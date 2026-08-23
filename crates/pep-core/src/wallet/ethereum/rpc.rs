use anyhow::{anyhow, Result};
use primitive_types::U256;
use reqwest::blocking::Client;
use serde_json::{json, Value};

use super::Address;

#[derive(Clone)]
pub struct Rpc {
    url: String,
    client: Client,
}

impl Rpc {

    /// Create RPC client.
    pub fn new(
        url: impl Into<String>,
    ) -> Self {

        Self {
            url: url.into(),
            client: Client::new(),
        }

    }

    fn call(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value> {

        let body = json!({

            "jsonrpc": "2.0",

            "id": 1,

            "method": method,

            "params": params,

        });

        let response =
            self.client
                .post(&self.url)
                .json(&body)
                .send()?;

        let value: Value =
            response.json()?;

        if let Some(error) =
            value.get("error")
        {

            return Err(anyhow!(
                "{}",
                error
            ));

        }

        Ok(
            value["result"].clone()
        )

    }

    pub fn chain_id(
        &self,
    ) -> Result<u64> {

        let value =
            self.call(
                "eth_chainId",
                json!([]),
            )?;

        let hex =
            value
                .as_str()
                .ok_or(anyhow!("invalid chain id"))?;

        Ok(
            u64::from_str_radix(
                hex.trim_start_matches("0x"),
                16,
            )?
        )

    }

    pub fn balance(
        &self,
        address: &Address,
    ) -> Result<U256> {

        let value =
            self.call(
                "eth_getBalance",
                json!([
                    address.to_string(),
                    "latest"
                ]),
            )?;

        let hex =
            value
                .as_str()
                .ok_or(anyhow!("invalid balance"))?;

        Ok(
            U256::from_str_radix(
                hex.trim_start_matches("0x"),
                16,
            )?
        )

    }

    pub fn nonce(
        &self,
        address: &Address,
    ) -> Result<u64> {

        let value =
            self.call(
                "eth_getTransactionCount",
                json!([
                    address.to_string(),
                    "latest"
                ]),
            )?;

        let hex =
            value
                .as_str()
                .ok_or(anyhow!("invalid nonce"))?;

        Ok(
            u64::from_str_radix(
                hex.trim_start_matches("0x"),
                16,
            )?
        )

    }

    pub fn gas_price(
        &self,
    ) -> Result<U256> {

        let value =
            self.call(
                "eth_gasPrice",
                json!([]),
            )?;

        let hex =
            value
                .as_str()
                .ok_or(anyhow!("invalid gas price"))?;

        Ok(
            U256::from_str_radix(
                hex.trim_start_matches("0x"),
                16,
            )?
        )

    }

    pub fn block_number(
        &self,
    ) -> Result<u64> {

        let value =
            self.call(
                "eth_blockNumber",
                json!([]),
            )?;

        let hex =
            value
                .as_str()
                .ok_or(anyhow!("invalid block"))?;

        Ok(
            u64::from_str_radix(
                hex.trim_start_matches("0x"),
                16,
            )?
        )

    }

    pub fn net_version(
        &self,
    ) -> Result<String> {

        let value =
            self.call(
                "net_version",
                json!([]),
            )?;

        Ok(
            value
                .as_str()
                .unwrap_or_default()
                .to_string()
        )

    }

    pub fn client_version(
        &self,
    ) -> Result<String> {

        let value =
            self.call(
                "web3_clientVersion",
                json!([]),
            )?;

        Ok(
            value
                .as_str()
                .unwrap_or_default()
                .to_string()
        )

    }
pub fn send_raw_transaction(
    &self,
    raw: &[u8],
) -> Result<String> {

    let hex =
        format!(
            "0x{}",
            hex::encode(raw),
        );

    let value =
        self.call(
            "eth_sendRawTransaction",
            json!([hex]),
        )?;

    Ok(
        value
            .as_str()
            .ok_or(anyhow!(
                "invalid tx hash"
            ))?
            .to_string()
    )

}
}