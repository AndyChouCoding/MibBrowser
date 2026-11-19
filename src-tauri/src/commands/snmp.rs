use crate::snmp::model::{ProfileInput, SnmpFeatures, ValidatedProfile};
use async_snmp::{Auth, Client, Oid};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadOperation {
    Get,
    GetNext,
    Walk,
    BulkWalk,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    kind: String,
    oid: String,
    value: String,
    duration_ms: u64,
}

#[derive(Default)]
pub struct ActiveQuery(std::sync::Mutex<Option<(String, tokio::sync::oneshot::Sender<()>)>>);

#[tauri::command]
pub fn cancel_query(state: tauri::State<'_, ActiveQuery>, id: String) -> Result<(), String> {
    cancel(&state, &id)
}

fn cancel(state: &ActiveQuery, id: &str) -> Result<(), String> {
    let mut active = state.0.lock().map_err(|_| "Query unavailable")?;
    if active.as_ref().is_some_and(|(key, _)| key == id) {
        if let Some((_, signal)) = active.take() {
            let _ = signal.send(());
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    id: String,
    host: String,
    port: u16,
    credentials: ReadCredentials,
    oid: String,
    operation: ReadOperation,
    timeout_ms: u64,
    retries: u8,
}

#[tauri::command]
pub async fn query_device(
    state: tauri::State<'_, ActiveQuery>,
    request: QueryRequest,
    on_rows: tauri::ipc::Channel<Vec<Binding>>,
) -> Result<usize, String> {
    execute_query(&state, request, |rows| {
        on_rows
            .send(rows)
            .map_err(|_| "Results channel closed".into())
    })
    .await
}

async fn execute_query(
    state: &ActiveQuery,
    request: QueryRequest,
    emit: impl FnMut(Vec<Binding>) -> Result<(), String>,
) -> Result<usize, String> {
    let QueryRequest {
        id,
        host,
        port,
        credentials,
        oid,
        operation,
        timeout_ms,
        retries,
    } = request;
    if host.trim().is_empty() || port == 0 {
        return Err("Enter a target and port".into());
    }
    if !(100..=120_000).contains(&timeout_ms) || retries > 10 {
        return Err("Timeout must be 100–120000 ms and retries must be 0–10".into());
    }
    let root: Oid = oid
        .trim()
        .trim_start_matches('.')
        .parse()
        .map_err(|_| "Invalid numeric OID")?;
    let retry = async_snmp::Retry::fixed(retries.into(), Duration::from_millis(100))
        .map_err(|_| "Invalid retry count")?;
    let builder = Client::builder((host.trim(), port), credentials.into_auth()?)
        .request_timeout(Duration::from_millis(timeout_ms))
        .retry(retry)
        .strict_source(true);
    let (signal, cancelled) = tokio::sync::oneshot::channel();
    {
        let mut active = state.0.lock().map_err(|_| "Query unavailable")?;
        if active.is_some() {
            return Err("A query is already running".into());
        }
        *active = Some((id.clone(), signal));
    }
    let result = tokio::select! {
        _ = cancelled => Err("Query cancelled".to_owned()),
        result = async {
            let client = builder.connect().await.map_err(|_| "Could not open SNMP transport")?;
            read_bindings(&client, root, operation, emit).await
        } => result
    };
    let mut active = state.0.lock().map_err(|_| "Query unavailable")?;
    if active.as_ref().is_some_and(|(key, _)| key == &id) {
        active.take();
    }
    result
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(
    tag = "version",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum ReadCredentials {
    V1 {
        community: String,
    },
    V2c {
        community: String,
    },
    V3 {
        username: String,
        context: String,
        level: crate::snmp::model::SecurityLevel,
        auth_password: String,
        privacy_password: String,
    },
}

impl ReadCredentials {
    pub(super) fn into_auth(self) -> Result<Auth, String> {
        use crate::snmp::model::SecurityLevel;
        use async_snmp::{AuthProtocol, PrivProtocol, UsmConfig};
        match self {
            Self::V1 { community } if !community.is_empty() => Ok(Auth::v1(community)),
            Self::V2c { community } if !community.is_empty() => Ok(Auth::v2c(community)),
            Self::V3 {
                username,
                context,
                level,
                auth_password,
                privacy_password,
            } => {
                if username.trim().is_empty() {
                    return Err("Enter a v3 username".into());
                }
                let base = UsmConfig::new(username).context_name(context);
                let config = match level {
                    SecurityLevel::None => Ok(base),
                    SecurityLevel::Auth if auth_password.len() >= 8 => {
                        base.auth(AuthProtocol::Sha256, auth_password)
                    }
                    SecurityLevel::AuthWithPrivacy
                        if auth_password.len() >= 8 && privacy_password.len() >= 8 =>
                    {
                        base.auth_priv(
                            AuthProtocol::Sha256,
                            auth_password,
                            PrivProtocol::Aes128,
                            privacy_password,
                        )
                    }
                    _ => return Err("USM passphrases must be at least 8 bytes".into()),
                }
                .map_err(|_| "Unable to initialize SNMPv3 security")?;
                Ok(config.into())
            }
            _ => Err("Enter a community".into()),
        }
    }
}

async fn read_bindings<T: async_snmp::Transport>(
    client: &Client<T>,
    root: Oid,
    operation: ReadOperation,
    mut emit: impl FnMut(Vec<Binding>) -> Result<(), String>,
) -> Result<usize, String> {
    let walking = matches!(operation, ReadOperation::Walk | ReadOperation::BulkWalk);
    let bulk = matches!(operation, ReadOperation::BulkWalk);
    if bulk && client.version() == async_snmp::Version::V1 {
        return Err("BULK WALK requires SNMPv2c or SNMPv3".into());
    }
    let mut cursor = root.clone();
    let prefix = format!("{root}.");
    let mut count = 0;
    loop {
        let start = Instant::now();
        let response = if bulk {
            client.get_bulk(std::slice::from_ref(&cursor), 0, 32).await
        } else {
            let result = match operation {
                ReadOperation::Get => client.get(&cursor).await,
                _ => client.get_next(&cursor).await,
            };
            match result {
                Ok(response) if !response.anomalies.is_empty() || response.varbinds.len() != 1 => {
                    return Err("Unexpected SNMP response binding".into())
                }
                Err(error)
                    if walking
                        && client.version() == async_snmp::Version::V1
                        && matches!(
                            error.exchange_source(),
                            async_snmp::Error::Snmp {
                                status: async_snmp::ErrorStatus::NoSuchName,
                                ..
                            }
                        ) =>
                {
                    return Ok(count)
                }
                other => other.map(|r| r.varbinds),
            }
        }
        .map_err(|_| "SNMP request failed. Check target, credentials and reachability.")?;
        if response.is_empty() {
            return Err("Agent returned an empty response".into());
        }
        let mut batch = Vec::new();
        let mut finished = !walking;
        for binding in response {
            let oid = binding.oid.to_string();
            if walking {
                if binding.value.is_exception() || !oid.starts_with(&prefix) {
                    finished = true;
                    break;
                }
                if binding.oid <= cursor {
                    return Err("Agent returned a non-increasing OID".into());
                }
                cursor = binding.oid;
            }
            batch.push(Binding {
                kind: binding.value.kind().as_str().to_owned(),
                oid,
                value: format!("{}", binding.value),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        count += batch.len();
        if !batch.is_empty() {
            emit(batch)?;
        }
        if finished {
            return Ok(count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_snmp::message::CommunityMessage;
    use async_snmp::pdu::{PduType, ResponsePdu};
    use async_snmp::{
        Candidate, DecodeConfig, RequestRegistration, Transport, Value, VarBind, Version,
    };
    use bytes::Bytes;
    use std::net::SocketAddr;

    struct FixtureAgent {
        subtree_size: u32,
    }
    impl Transport for FixtureAgent {
        fn peer_addr(&self) -> SocketAddr {
            ([127, 0, 0, 1], 161).into()
        }
        fn local_addr(&self) -> SocketAddr {
            ([127, 0, 0, 1], 0).into()
        }
        fn is_reliable(&self) -> bool {
            true
        }
        async fn send(&self, _: &[u8]) -> async_snmp::Result<()> {
            unreachable!()
        }
        async fn request_with<T, F>(
            &self,
            packet: &[u8],
            _: RequestRegistration,
            mut validate: F,
        ) -> async_snmp::Result<T>
        where
            T: Send,
            F: FnMut(Bytes, SocketAddr) -> async_snmp::Result<Candidate<T>> + Send,
        {
            let msg =
                CommunityMessage::decode(Bytes::copy_from_slice(packet), DecodeConfig::default())?
                    .value;
            let request = msg.pdu().standard().unwrap();
            let cursor = &request.varbinds()[0].oid;
            let successors = (1..=self.subtree_size)
                .map(|i| format!("1.3.6.{i}").parse::<Oid>().unwrap())
                .chain(std::iter::once("1.3.7".parse().unwrap()))
                .filter(|o| o > cursor);
            let oids = match request.pdu_type() {
                PduType::GetRequest => vec![cursor.clone()],
                PduType::GetNextRequest => successors.take(1).collect(),
                PduType::GetBulkRequest if msg.version() == Version::V2c => {
                    successors.take(32).collect()
                }
                _ => panic!("unexpected request"),
            };
            let reply = ResponsePdu::success(
                msg.version(),
                request.request_id(),
                oids.into_iter()
                    .map(|oid| VarBind::new(oid, Value::Integer(42)))
                    .collect(),
            )?
            .into_raw();
            let bytes = match msg.version() {
                Version::V1 => CommunityMessage::v1("fixture", reply)?,
                _ => CommunityMessage::v2c("fixture", reply)?,
            }
            .encode()?;
            match validate(bytes, self.peer_addr())? {
                Candidate::Accept(result) => Ok(result),
                _ => panic!("fixture response rejected"),
            }
        }
    }

    #[tokio::test]
    #[ignore = "run through scripts/test-local-agent.py with an isolated Agent"]
    async fn isolated_agent_read_versions() {
        let port = std::env::var("MIB_TEST_PORT")
            .unwrap()
            .parse::<u16>()
            .unwrap();
        let community = std::env::var("MIB_TEST_COMMUNITY").unwrap();
        let secret = std::env::var("MIB_TEST_PASSPHRASE").unwrap();
        let mut modes = vec![Auth::v1(community.clone()), Auth::v2c(community)];
        use crate::snmp::model::SecurityLevel;
        for (username, level) in [
            ("fixtureNone", SecurityLevel::None),
            ("fixtureAuth", SecurityLevel::Auth),
            ("fixturePriv", SecurityLevel::AuthWithPrivacy),
        ] {
            modes.push(
                ReadCredentials::V3 {
                    username: username.into(),
                    level,
                    context: String::new(),
                    auth_password: secret.clone(),
                    privacy_password: secret.clone(),
                }
                .into_auth()
                .unwrap(),
            );
        }
        for auth in modes {
            let client = Client::builder(("127.0.0.1", port), auth)
                .request_timeout(Duration::from_secs(2))
                .retry(async_snmp::Retry::none())
                .strict_source(true)
                .connect()
                .await
                .unwrap();
            let scalar = "1.3.6.1.2.1.1.3.0";
            let mut rows = Vec::new();
            read_bindings(
                &client,
                scalar.parse().unwrap(),
                ReadOperation::Get,
                |batch| {
                    rows.extend(batch);
                    Ok(())
                },
            )
            .await
            .unwrap();
            assert_eq!(rows[0].oid, scalar);
            for op in [
                ReadOperation::GetNext,
                ReadOperation::Walk,
                ReadOperation::BulkWalk,
            ] {
                if client.version() == Version::V1 && matches!(op, ReadOperation::BulkWalk) {
                    continue;
                }
                let n = read_bindings(&client, "1.3.6.1.2.1.1".parse().unwrap(), op, |batch| {
                    assert!(batch.iter().all(|b| b.oid.starts_with("1.3.6.1.2.1.1.")));
                    Ok(())
                })
                .await
                .unwrap();
                assert!(n > 0);
            }
        }
    }

    #[tokio::test]
    #[ignore = "run through scripts/test-local-agent.py with an isolated Agent"]
    async fn isolated_agent_cancel_timeout_and_retry() {
        let port = std::env::var("MIB_TEST_PORT").unwrap().parse().unwrap();
        let good = std::env::var("MIB_TEST_COMMUNITY").unwrap();
        let state = ActiveQuery::default();
        let request = |community: String, retries| QueryRequest {
            id: "fixture".into(),
            host: "127.0.0.1".into(),
            port,
            credentials: ReadCredentials::V2c { community },
            oid: "1.3.6.1.2.1.1.3.0".into(),
            operation: ReadOperation::Get,
            timeout_ms: 250,
            retries,
        };
        let (result, signal) = tokio::join!(
            execute_query(&state, request("invalid".into(), 1), |_| Ok(())),
            async {
                tokio::time::sleep(Duration::from_millis(20)).await;
                cancel(&state, "fixture")
            }
        );
        signal.unwrap();
        assert_eq!(result.unwrap_err(), "Query cancelled");
        let start = Instant::now();
        assert!(
            execute_query(&state, request("invalid".into(), 1), |_| Ok(()))
                .await
                .is_err()
        );
        assert!(start.elapsed() >= Duration::from_millis(500));
        assert_eq!(
            execute_query(&state, request(good, 0), |_| Ok(()))
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn v1_walk_uses_getnext_and_rejects_bulk() {
        let client = async_snmp::ClientBuilder::new(Auth::v1("fixture"))
            .build_with_transport(FixtureAgent { subtree_size: 2 })
            .unwrap();
        let root: Oid = "1.3.6".parse().unwrap();
        assert_eq!(
            read_bindings(&client, root.clone(), ReadOperation::Walk, |_| Ok(()))
                .await
                .unwrap(),
            2
        );
        assert!(
            read_bindings(&client, root, ReadOperation::BulkWalk, |_| panic!(
                "unexpected batch"
            ))
            .await
            .unwrap_err()
            .contains("requires")
        );
    }

    #[tokio::test]
    async fn streams_large_bulk_walk_in_bounded_batches() {
        let client = async_snmp::ClientBuilder::new(Auth::v2c("fixture"))
            .build_with_transport(FixtureAgent { subtree_size: 2048 })
            .unwrap();
        let mut batch_count = 0;
        let rows = read_bindings(
            &client,
            "1.3.6".parse().unwrap(),
            ReadOperation::BulkWalk,
            |batch| {
                assert_eq!(batch.len(), 32);
                batch_count += 1;
                Ok(())
            },
        )
        .await
        .unwrap();
        assert_eq!((rows, batch_count), (2048, 64));
    }

    #[tokio::test]
    async fn queries_encoded_agent_responses() {
        let client = async_snmp::ClientBuilder::new(Auth::v2c("fixture"))
            .build_with_transport(FixtureAgent { subtree_size: 2 })
            .unwrap();
        for (operation, expected) in [
            (ReadOperation::Get, vec!["1.3.6"]),
            (ReadOperation::GetNext, vec!["1.3.6.1"]),
            (ReadOperation::BulkWalk, vec!["1.3.6.1", "1.3.6.2"]),
            (ReadOperation::Walk, vec!["1.3.6.1", "1.3.6.2"]),
        ] {
            let mut rows = Vec::new();
            read_bindings(&client, "1.3.6".parse().unwrap(), operation, |batch| {
                rows.extend(batch);
                Ok(())
            })
            .await
            .unwrap();
            assert_eq!(
                rows.iter().map(|row| row.oid.as_str()).collect::<Vec<_>>(),
                expected
            );
            assert!(rows.iter().all(|row| row.value.contains("42")));
        }
    }
}

#[tauri::command]
pub fn supported_snmp_features() -> SnmpFeatures {
    SnmpFeatures::planned_mvp()
}

#[tauri::command]
pub fn validate_snmp_profile(profile: ProfileInput) -> Result<ValidatedProfile, String> {
    profile.validate().map_err(|error| error.to_string())
}
