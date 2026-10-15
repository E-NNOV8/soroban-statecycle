//! SCF Governance

use crate::{
    SCFGovernanceTrait, SCFMembership, SCFMembershipArgs, SCFMembershipClient, SCFTokenTrait,
    errors, events, types,
};
use soroban_sdk::{
    Address, Env, I256, InvokeError, String, Symbol, Vec, contractimpl, panic_with_error, vec,
};

#[contractimpl]
impl SCFGovernanceTrait for SCFMembership {
    fn trait_value(e: &Env, token_id: u32, trait_key: String) -> i128 {
        validate_trait_key(e, &trait_key);
        if trait_key == String::from_str(e, "role") {
            let role: i128 = e
                .storage()
                .persistent()
                .get(&types::NFTStorageKey::Role(token_id))
                .unwrap_or_else(|| {
                    panic_with_error!(&e, errors::NonFungibleTokenError::NonExistentToken)
                });
            role
        } else {
            get_nqg(e, token_id)
        }
    }

    fn trait_values(e: &Env, token_id: u32, trait_keys: Vec<String>) -> Vec<i128> {
        let mut values = Vec::new(e);
        for trait_key in trait_keys.iter() {
            values.push_back(Self::trait_value(e, token_id, trait_key));
        }
        values
    }

    fn set_trait(e: &Env, token_id: u32, trait_key: String, new_value: i128) {
        let admin: Address = e.storage().instance().get(&types::DataKey::Admin).unwrap();
        admin.require_auth();

        let role_key = String::from_str(e, "role");
        if trait_key != role_key {
            panic_with_error!(e, errors::NonFungibleTokenError::TraitUnSettable);
        }

        if !(0..=4).contains(&new_value) {
            panic_with_error!(&e, errors::NonFungibleTokenError::RoleDoesNotExist)
        }

        e.storage()
            .persistent()
            .set(&types::NFTStorageKey::Role(token_id), &new_value);

        events::SetTrait {
            trait_key,
            token_id,
            new_value,
        }
        .publish(e);
    }

    fn trait_metadata_uri(e: &Env) -> String {
        e.storage()
            .instance()
            .get(&types::DataKey::UriTrait)
            .unwrap()
    }

    fn governance(e: &Env, token_id: u32) -> types::Governance {
        let role = match Self::trait_value(e, token_id, String::from_str(e, "role")) {
            3 => types::Role::Pilot,
            2 => types::Role::Navigator,
            1 => types::Role::Pathfinder,
            0 => types::Role::Verified,
            _ => panic_with_error!(&e, errors::NonFungibleTokenError::RoleDoesNotExist),
        };

        types::Governance {
            role,
            nqg: Self::trait_value(e, token_id, String::from_str(e, "nqg")),
        }
    }
}

fn validate_trait_key(e: &Env, trait_key: &String) {
    let role_key = String::from_str(e, "role");
    let nqg_key = String::from_str(e, "nqg");
    if trait_key != &role_key && trait_key != &nqg_key {
        panic_with_error!(e, errors::NonFungibleTokenError::TraitDoesNotExist);
    }
}

fn get_nqg(e: &Env, token_id: u32) -> i128 {
    let owner = SCFMembership::owner_of(e, token_id);

    let nqg_contract_address = e
        .storage()
        .instance()
        .get(&types::DataKey::NqgContract)
        .unwrap();
    let r = e.try_invoke_contract::<I256, InvokeError>(
        &nqg_contract_address,
        &Symbol::new(e, "get_voting_power_for_user"),
        vec![e, owner.to_string().to_val()],
    );
    let nqg: I256 = match r {
        Ok(Ok(v)) => v,
        _ => I256::from_i128(e, 0),
    };
    let scaled = nqg.div(&I256::from_i128(e, 10_i128.pow(12)));
    scaled.to_i128().unwrap()
}
