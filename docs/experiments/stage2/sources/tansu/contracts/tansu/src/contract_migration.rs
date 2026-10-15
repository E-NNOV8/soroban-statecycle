use soroban_sdk::{Address, Bytes, Env, String, Vec, contractimpl, contracttype};

use crate::{MigrationTrait, Tansu, TansuArgs, TansuClient, types};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectV1 {
    pub name: String,
    pub config: types::Config,
    pub maintainers: Vec<Address>,
}

#[contractimpl]
impl MigrationTrait for Tansu {
    fn projects_migration(env: Env, admin: Address, names: Vec<String>) {
        crate::contract_tansu::auth_admin(&env, &admin);

        for name in names {
            let key: Bytes = env.crypto().keccak256(&name.to_bytes()).into();
            let key_ = types::ProjectKey::Key(key.clone());

            let project_v1 = env
                .storage()
                .persistent()
                .get::<types::ProjectKey, ProjectV1>(&key_)
                .expect("Migration");

            let project_v2 = types::Project {
                name: project_v1.name,
                config: project_v1.config,
                maintainers: project_v1.maintainers,
                sub_projects: None,
            };

            env.storage().persistent().set(&key_, &project_v2);
        }
    }
}
