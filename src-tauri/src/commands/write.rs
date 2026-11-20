use super::snmp::ReadCredentials;
use async_snmp::{Client, Oid, Retry, Value};
use mib_rs::{
    mib::Object,
    types::{Access, BaseType, Kind},
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    host: String,
    port: u16,
    credentials: ReadCredentials,
    paths: Vec<PathBuf>,
    object_oid: String,
    instance: String,
    input: String,
    timeout_ms: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    token: String,
    target: String,
    oid: String,
    current: String,
    proposed: String,
}

struct Pending {
    token: String,
    client: Client,
    oid: Oid,
    previous: Value,
    next: Value,
    created: Instant,
}
#[derive(Default)]
pub struct PendingSet(Mutex<Option<Pending>>);

fn validate_value(object: Object<'_>, input: &str) -> Result<Value, String> {
    if !matches!(object.access(), Access::ReadWrite | Access::ReadCreate) {
        return Err("Object does not allow a confirmed read/write operation".into());
    }
    let base = object.ty().ok_or("Unknown object syntax")?.effective_base();
    let integer = || {
        input
            .parse::<i32>()
            .map_err(|_| "Expected a signed 32-bit integer")
    };
    let unsigned = || {
        input
            .parse::<u32>()
            .map_err(|_| "Expected an unsigned 32-bit integer")
    };
    let (value, number) = match base {
        BaseType::Integer32 => {
            let n = integer()?;
            if !object.effective_enums().is_empty()
                && !object
                    .effective_enums()
                    .iter()
                    .any(|e| e.value == i64::from(n))
            {
                return Err("Value is outside the MIB enumeration".into());
            }
            (Value::Integer(n), Some(i64::from(n)))
        }
        BaseType::Gauge32 | BaseType::Unsigned32 => {
            let n = unsigned()?;
            (Value::Gauge32(n), Some(i64::from(n)))
        }
        BaseType::TimeTicks => {
            let n = unsigned()?;
            (Value::TimeTicks(n), Some(i64::from(n)))
        }
        BaseType::OctetString if input.len() <= 65535 => (
            Value::OctetString(input.as_bytes().to_vec().into()),
            Some(input.len() as i64),
        ),
        BaseType::IpAddress => (
            Value::IpAddress(
                input
                    .parse::<std::net::Ipv4Addr>()
                    .map_err(|_| "Invalid IPv4 address")?
                    .octets(),
            ),
            None,
        ),
        BaseType::ObjectIdentifier => (
            Value::ObjectIdentifier(input.parse().map_err(|_| "Invalid OID value")?),
            None,
        ),
        _ => return Err("Unsupported SET syntax or value length".into()),
    };
    if let Some(n) = number {
        let (constrained, ranges) = if base == BaseType::OctetString {
            (
                object.effective_sizes_constrained(),
                object.effective_sizes(),
            )
        } else {
            (
                object.effective_ranges_constrained(),
                object.effective_ranges(),
            )
        };
        if constrained
            && !ranges.iter().any(|r| {
                r.min.as_i64().is_some_and(|min| min <= n)
                    && r.max.as_i64().is_some_and(|max| n <= max)
            })
        {
            return Err("Value is outside the permitted MIB range or size".into());
        }
    }
    Ok(value)
}

async fn read_current(client: &Client, oid: &Oid) -> Result<Value, String> {
    let response = client
        .get(oid)
        .await
        .map_err(|_| "Read-back request failed")?;
    match response.varbinds.as_slice() {
        [binding]
            if response.anomalies.is_empty()
                && binding.oid == *oid
                && !binding.value.is_exception() =>
        {
            Ok(binding.value.clone())
        }
        _ => Err("Device did not return the requested instance".into()),
    }
}

#[tauri::command]
pub async fn prepare_set(
    state: tauri::State<'_, PendingSet>,
    proposal: Proposal,
) -> Result<Preview, String> {
    prepare(&state, proposal).await
}

async fn prepare(state: &PendingSet, proposal: Proposal) -> Result<Preview, String> {
    let Proposal {
        host,
        port,
        credentials,
        paths,
        object_oid,
        instance,
        input,
        timeout_ms,
    } = proposal;
    if !(100..=120000).contains(&timeout_ms) || host.trim().is_empty() || port == 0 {
        return Err("Invalid target or timeout".into());
    }
    let (oid, next) = tauri::async_runtime::spawn_blocking(move || {
        let mib = crate::mib::engine::load_model(paths).map_err(|e| e.to_string())?;
        let node = mib
            .resolve_node(&object_oid)
            .ok_or("Selected object is missing")?;
        if node.oid().to_string() != object_oid {
            return Err("Select an exact MIB object".into());
        }
        let object = node.object().ok_or("Select a writable object")?;
        match object.kind() {
            Kind::Scalar if instance == "0" => (),
            Kind::Column => {
                use mib_rs::mib::index::{DecodeOptions, IndexSchema};
                let arcs: Vec<u32> = instance
                    .split('.')
                    .map(str::parse)
                    .collect::<Result<_, _>>()
                    .map_err(|_| "Invalid table index")?;
                let schema =
                    IndexSchema::compile(object).map_err(|_| "Cannot validate the table index")?;
                schema
                    .decode_exact(&arcs, DecodeOptions::new(128))
                    .map_err(|_| "Index does not match the table definition")?;
            }
            _ => return Err("Invalid object instance".into()),
        }
        let oid = format!("{object_oid}.{instance}")
            .parse::<Oid>()
            .map_err(|_| "Invalid numeric instance")?;
        Ok::<_, String>((oid, validate_value(object, &input)?))
    })
    .await
    .map_err(|_| "MIB worker unavailable")??;
    let client = Client::builder((host.trim(), port), credentials.into_auth()?)
        .strict_source(true)
        .request_timeout(Duration::from_millis(timeout_ms))
        .retry(Retry::none())
        .connect()
        .await
        .map_err(|_| "Could not open SNMP transport")?;
    let previous = read_current(&client, &oid).await?;
    if previous.kind() != next.kind() {
        return Err("Device type differs from MIB syntax".into());
    }
    let token = uuid::Uuid::new_v4().to_string();
    let preview = Preview {
        token: token.clone(),
        target: client.peer_addr().to_string(),
        oid: oid.to_string(),
        current: previous.to_string(),
        proposed: next.to_string(),
    };
    *state.0.lock().map_err(|_| "SET unavailable")? = Some(Pending {
        token,
        client,
        oid,
        previous,
        next,
        created: Instant::now(),
    });
    Ok(preview)
}

#[tauri::command]
pub async fn finish_set(
    state: tauri::State<'_, PendingSet>,
    token: String,
    confirm: bool,
) -> Result<String, String> {
    finish(&state, token, confirm).await
}

async fn finish(state: &PendingSet, token: String, confirm: bool) -> Result<String, String> {
    let p = {
        let mut state = state.0.lock().map_err(|_| "SET unavailable")?;
        if !state.as_ref().is_some_and(|p| p.token == token) {
            return Err("Preview expired".into());
        }
        state.take().ok_or("Preview expired")?
    };
    if !confirm {
        return Ok("SET cancelled".into());
    }
    if p.created.elapsed() > Duration::from_secs(60) {
        return Err("Preview expired. Review the current value again.".into());
    }
    if read_current(&p.client, &p.oid).await? != p.previous {
        return Err("Value changed. Review before writing.".into());
    }
    let result = p
        .client
        .set(&p.oid, p.next.clone())
        .await
        .map_err(|_| "SET outcome unknown. Read the instance before retrying.")?;
    if !result.anomalies.is_empty() {
        return Err("Unexpected SET response. Read the instance before retrying.".into());
    }
    let actual = read_current(&p.client, &p.oid)
        .await
        .map_err(|_| "SET acknowledged, read-back unavailable")?;
    if actual != p.next {
        return Err("Read-back differs from the proposed value".into());
    }
    Ok(format!("SET verified · {} = {actual}", p.oid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "run through scripts/test-local-agent.py with an isolated writable Agent"]
    async fn isolated_agent_lost_set_reply() {
        use async_snmp::{message::CommunityMessage, pdu::PduType, DecodeConfig};
        use bytes::Bytes;
        use tokio::net::UdpSocket;
        let port: u16 = std::env::var("MIB_TEST_PORT").unwrap().parse().unwrap();
        let community = std::env::var("MIB_TEST_COMMUNITY").unwrap();
        let proxy = UdpSocket::bind(("127.0.0.1", 0)).await.unwrap();
        let proxy_port = proxy.local_addr().unwrap().port();
        let upstream = UdpSocket::bind(("127.0.0.1", 0)).await.unwrap();
        upstream.connect(("127.0.0.1", port)).await.unwrap();
        let relay = tokio::spawn(async move {
            let mut buffer = [0u8; 8192];
            loop {
                let (len, peer) = proxy.recv_from(&mut buffer).await.unwrap();
                let msg = CommunityMessage::decode(
                    Bytes::copy_from_slice(&buffer[..len]),
                    DecodeConfig::default(),
                )
                .unwrap()
                .value;
                upstream.send(&buffer[..len]).await.unwrap();
                let n = upstream.recv(&mut buffer).await.unwrap();
                if msg.pdu().pdu_type() == PduType::SetRequest {
                    break;
                }
                proxy.send_to(&buffer[..n], peer).await.unwrap();
            }
        });
        let client = Client::builder(
            ("127.0.0.1", port),
            async_snmp::Auth::v2c(community.clone()),
        )
        .connect()
        .await
        .unwrap();
        let oid = "1.3.6.1.2.1.1.6.0".parse().unwrap();
        let previous = read_current(&client, &oid).await.unwrap();
        let state = PendingSet::default();
        let preview = prepare(
            &state,
            Proposal {
                host: "127.0.0.1".into(),
                port: proxy_port,
                credentials: ReadCredentials::V2c { community },
                paths: vec![std::env::var("MIB_TEST_MIB").unwrap().into()],
                object_oid: "1.3.6.1.2.1.1.6".into(),
                instance: "0".into(),
                input: "lost-reply-test".into(),
                timeout_ms: 250,
            },
        )
        .await
        .unwrap();
        let error = finish(&state, preview.token, true).await.unwrap_err();
        relay.await.unwrap();
        assert!(error.contains("outcome unknown"));
        assert_eq!(
            read_current(&client, &oid).await.unwrap(),
            Value::OctetString(Bytes::from_static(b"lost-reply-test"))
        );
        client.set(&oid, previous.clone()).await.unwrap();
        assert_eq!(read_current(&client, &oid).await.unwrap(), previous);
    }

    #[tokio::test]
    #[ignore = "run through scripts/test-local-agent.py with an isolated writable Agent"]
    async fn isolated_agent_guarded_write() {
        let port = std::env::var("MIB_TEST_PORT").unwrap().parse().unwrap();
        let community = std::env::var("MIB_TEST_COMMUNITY").unwrap();
        let paths = std::env::var("MIB_TEST_MIB").unwrap();
        let proposal = |value: &str| Proposal {
            host: "127.0.0.1".into(),
            port,
            credentials: ReadCredentials::V2c {
                community: community.clone(),
            },
            paths: vec![paths.clone().into()],
            object_oid: "1.3.6.1.2.1.1.6".into(),
            instance: "0".into(),
            input: value.into(),
            timeout_ms: 2000,
        };
        let state = PendingSet::default();
        let oid: Oid = "1.3.6.1.2.1.1.6.0".parse().unwrap();
        let client = Client::builder(
            ("127.0.0.1", port),
            async_snmp::Auth::v2c(community.clone()),
        )
        .connect()
        .await
        .unwrap();
        let previous = read_current(&client, &oid).await.unwrap();
        let Value::OctetString(bytes) = &previous else {
            panic!("expected location string")
        };
        let saved = std::str::from_utf8(bytes).unwrap();
        let preview = prepare(&state, proposal("isolated test")).await.unwrap();
        finish(&state, preview.token, false).await.unwrap();
        assert_eq!(read_current(&client, &oid).await.unwrap(), previous);
        let preview = prepare(&state, proposal("isolated test")).await.unwrap();
        finish(&state, preview.token.clone(), true).await.unwrap();
        assert!(finish(&state, preview.token, true).await.is_err());
        let restore = prepare(&state, proposal(saved)).await.unwrap();
        finish(&state, restore.token, true).await.unwrap();
        assert_eq!(read_current(&client, &oid).await.unwrap(), previous);
    }
}
