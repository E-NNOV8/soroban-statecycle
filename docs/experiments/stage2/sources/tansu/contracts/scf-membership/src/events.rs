use soroban_sdk::{Address, String, contractevent};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mint {
    #[topic]
    pub to: Address,
    pub token_id: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetTrait {
    #[topic]
    pub trait_key: String,
    pub token_id: u32,
    pub new_value: i128,
}
