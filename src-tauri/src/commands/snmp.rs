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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    oid: String,
    value: String,
    duration_ms: u64,
}

#[tauri::command]
pub async fn query_v2c(
    host: String,
    port: u16,
    community: String,
    oid: String,
    operation: ReadOperation,
) -> Result<Vec<Binding>, String> {
    if host.trim().is_empty() || port == 0 || community.is_empty() {
        return Err("Enter a target, port and community".into());
    }
    let root: Oid = oid
        .trim()
        .trim_start_matches('.')
        .parse()
        .map_err(|_| "Invalid numeric OID")?;
    tokio::time::timeout(Duration::from_secs(30), async {
        let client = Client::builder((host.trim(), port), Auth::v2c(community))
            .strict_source(true)
            .connect()
            .await
            .map_err(|_| "Could not open SNMP transport")?;
        read_bindings(&client, root, operation).await
    })
    .await
    .map_err(|_| "Query timed out")?
}

async fn read_bindings<T: async_snmp::Transport>(
    client: &Client<T>,
    root: Oid,
    operation: ReadOperation,
) -> Result<Vec<Binding>, String> {
    let mut cursor = root.clone();
    let prefix = format!("{root}.");
    let mut rows = Vec::new();
    loop {
        let start = Instant::now();
        let response = match operation {
            ReadOperation::Get => client.get(&cursor).await,
            _ => client.get_next(&cursor).await,
        }
        .map_err(|_| "SNMP request failed. Check target, community and reachability.")?;
        let [binding] = response.varbinds.as_slice() else {
            return Err("Unexpected SNMP response shape".into());
        };
        if !response.anomalies.is_empty() {
            return Err("Unexpected SNMP response binding".into());
        }
        let oid = binding.oid.to_string();
        if matches!(operation, ReadOperation::Walk) {
            if binding.value.is_exception() || !oid.starts_with(&prefix) {
                break;
            }
            if binding.oid <= cursor {
                return Err("Agent returned a non-increasing OID".into());
            }
            if rows.len() >= 1024 {
                return Err("Walk exceeded 1024 results. Select a smaller subtree.".into());
            }
        }
        rows.push(Binding {
            oid,
            value: format!("{}", binding.value),
            duration_ms: start.elapsed().as_millis() as u64,
        });
        if !matches!(operation, ReadOperation::Walk) {
            break;
        }
        cursor = binding.oid.clone();
    }
    Ok(rows)
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

    struct FixtureAgent;
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
            let oid = match request.pdu_type() {
                PduType::GetRequest => cursor.clone(),
                PduType::GetNextRequest => ["1.3.6.1", "1.3.6.2", "1.3.7"]
                    .into_iter()
                    .map(|s| s.parse::<Oid>().unwrap())
                    .find(|o| o > cursor)
                    .unwrap(),
                _ => panic!("unexpected request"),
            };
            let reply = ResponsePdu::success(
                Version::V2c,
                request.request_id(),
                vec![VarBind::new(oid, Value::Integer(42))],
            )?;
            let bytes = CommunityMessage::v2c("fixture", reply.into_raw())?.encode()?;
            match validate(bytes, self.peer_addr())? {
                Candidate::Accept(result) => Ok(result),
                _ => panic!("fixture response rejected"),
            }
        }
    }

    #[tokio::test]
    async fn queries_encoded_agent_responses() {
        let client = async_snmp::ClientBuilder::new(Auth::v2c("fixture"))
            .build_with_transport(FixtureAgent)
            .unwrap();
        for (operation, expected) in [
            (ReadOperation::Get, vec!["1.3.6"]),
            (ReadOperation::GetNext, vec!["1.3.6.1"]),
            (ReadOperation::Walk, vec!["1.3.6.1", "1.3.6.2"]),
        ] {
            let rows = read_bindings(&client, "1.3.6".parse().unwrap(), operation)
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
