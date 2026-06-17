use soroban_sdk::{contracttype, symbol_short, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NonceRecord {
    pub chain_id: u32,
    pub nonce: u64,
    pub used: bool,
}

pub struct NonceManager;

impl NonceManager {
    const NONCE_LIST_KEY: Symbol = symbol_short!("_nonces");
    const CHAIN_KEY: Symbol = symbol_short!("_chain");

    pub fn record_and_verify(env: &Env, chain_id: u32, nonce: u64) -> Result<(), ()> {
        let required_chain: Option<u32> = env
            .storage()
            .persistent()
            .get(&Self::CHAIN_KEY)
            .unwrap_or(None);

        if let Some(expected) = required_chain {
            if chain_id != expected {
                panic!("INVALID_CHAIN_ID");
            }
        }

        let mut nonces: Vec<NonceRecord> = env
            .storage()
            .persistent()
            .get(&Self::NONCE_LIST_KEY)
            .unwrap_or_else(|| Vec::new(env));

        let mut found = false;
        for i in 0..nonces.len() {
            let mut record = nonces.get_unchecked(i);
            if record.chain_id == chain_id && record.nonce == nonce {
                if record.used {
                    panic!("REPLAY_DETECTED");
                }
                record.used = true;
                nonces.set(i, record);
                found = true;
                break;
            }
        }

        if !found {
            let new_record = NonceRecord {
                chain_id,
                nonce,
                used: true,
            };
            nonces.push_back(new_record);
        }

        env.storage()
            .persistent()
            .set(&Self::NONCE_LIST_KEY, &nonces);
        Ok(())
    }

    pub fn set_chain_id(env: &Env, chain_id: u32) {
        env.storage().persistent().set(&Self::CHAIN_KEY, &chain_id);
    }

    pub fn get_last_nonce(env: &Env, chain_id: u32) -> Option<u64> {
        let nonces: Vec<NonceRecord> = env
            .storage()
            .persistent()
            .get(&Self::NONCE_LIST_KEY)
            .unwrap_or_else(|| Vec::new(env));

        let mut max_nonce: Option<u64> = None;

        for i in 0..nonces.len() {
            let record = nonces.get_unchecked(i);
            if record.chain_id == chain_id {
                max_nonce = Some(record.nonce);
            }
        }

        max_nonce
    }

    pub fn enforce_sequential_nonce(env: &Env, chain_id: u32, nonce: u64) {
        let last = Self::get_last_nonce(env, chain_id);
        if let Some(expected) = last {
            if nonce <= expected {
                panic!("NONCE_OUT_OF_ORDER");
            }
        }
    }
}