// Copyright 2019 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! This module implements node maintenance actions.

use std::{collections::HashMap, path::{Path, PathBuf}, str::FromStr};

use super::{
    internal::{CollectedCommand, Command, Feedback},
    Argument, CommandName, Context,
};
use crate::blockchain::Schema;
use crate::helpers::config::ConfigFile;
use crate::node::NodeConfig;
use exonum_merkledb::{Database, DbOptions, RocksDB};
use crate::helpers::fabric::password::{PassInputMethod, SecretKeyType};

// Context entry for the path to the node config.
const NODE_CONFIG_PATH: &str = "NODE_CONFIG_PATH";
// Context entry for the path to the database.
const DATABASE_PATH: &str = "DATABASE_PATH";
// Context entry for the type of action to be performed.
const MAINTENANCE_ACTION_PATH: &str = "MAINTENANCE_ACTION_PATH";

const CONSENSUS_KEY_PASS_METHOD: &str = "CONSENSUS_KEY_PASS_METHOD";

const SERVICE_KEY_PASS_METHOD: &str = "SERVICE_KEY_PASS_METHOD";

/// Maintenance command. Supported actions:
///
/// - `clear-cache` - clear message cache.
#[derive(Debug)]
pub struct Maintenance;

impl Maintenance {
    fn node_config(ctx: &Context) -> NodeConfig {
        let path = ctx
            .arg::<String>(NODE_CONFIG_PATH)
            .unwrap_or_else(|_| panic!("{} not found.", NODE_CONFIG_PATH));
        let run_config: NodeConfig<PathBuf> = ConfigFile::load(path.clone()).expect("Can't load node config file");

        let consensus_passphrase = {
            let consensus_pass_method = ctx
                .arg::<String>(CONSENSUS_KEY_PASS_METHOD)
                .unwrap_or_else(|_| panic!("{} not found.", NODE_CONFIG_PATH));

            PassInputMethod::from_str(&consensus_pass_method)
                .expect("Incorrect passphrase input method for consensus key.")
                .get_passphrase(SecretKeyType::Consensus, true)
        };

        let service_passphrase = {
            let service_pass_method = ctx
                .arg::<String>(SERVICE_KEY_PASS_METHOD)
                .unwrap_or_else(|_| panic!("{} not found.", NODE_CONFIG_PATH));

            PassInputMethod::from_str(&service_pass_method)
                .expect("Incorrect passphrase input method for service key.")
                .get_passphrase(SecretKeyType::Service, true)
        };

        run_config.read_secret_keys(
            &path,
            consensus_passphrase.as_bytes(),
            service_passphrase.as_bytes(),
        )
    }

    fn database(ctx: &Context, options: &DbOptions) -> Box<dyn Database> {
        let path = ctx
            .arg::<String>(DATABASE_PATH)
            .unwrap_or_else(|_| panic!("{} not found.", DATABASE_PATH));
        Box::new(RocksDB::open(Path::new(&path), options).expect("Can't load database file"))
    }

    fn clear_cache(context: &Context) {
        info!("Clearing node cache");

        let config = Self::node_config(context);
        let db = Self::database(context, &config.database);
        let fork = db.fork();
        Schema::new(&fork).consensus_messages_cache().clear();
        db.merge_sync(fork.into_patch()).expect("Can't clear cache");

        info!("Cache cleared successfully");
    }
}

impl Command for Maintenance {
    fn args(&self) -> Vec<Argument> {
        vec![
            Argument::new_named(
                NODE_CONFIG_PATH,
                true,
                "Path to node configuration file.",
                "c",
                "node-config",
                false,
            ),
            Argument::new_named(
                DATABASE_PATH,
                true,
                "Use database with the given path.",
                "d",
                "db-path",
                false,
            ),
            Argument::new_named(
                MAINTENANCE_ACTION_PATH,
                true,
                "Action to be performed during maintenance.",
                "a",
                "action",
                false,
            ),
            Argument::new_named(
                CONSENSUS_KEY_PASS_METHOD,
                false,
                "Passphrase entry method for consensus key.\n\
                 Possible values are: stdin, env{:ENV_VAR_NAME}, pass:PASSWORD (default: stdin)\n\
                 If ENV_VAR_NAME is not specified $EXONUM_CONSENSUS_PASS is used",
                None,
                "consensus-key-pass",
                false,
            ),
            Argument::new_named(
                SERVICE_KEY_PASS_METHOD,
                false,
                "Passphrase entry method for service key.\n\
                 Possible values are: stdin, env{:ENV_VAR_NAME}, pass:PASSWORD (default: stdin)\n\
                 If ENV_VAR_NAME is not specified $EXONUM_SERVICE_PASS is used",
                None,
                "service-key-pass",
                false,
            ),
        ]
    }

    fn name(&self) -> CommandName {
        "maintenance"
    }

    fn about(&self) -> &str {
        "Maintenance module. Available actions: clear-cache."
    }

    fn execute(
        &self,
        _commands: &HashMap<CommandName, CollectedCommand>,
        context: Context,
        _: &dyn Fn(Context) -> Context,
    ) -> Feedback {
        let action = context
            .arg::<String>(MAINTENANCE_ACTION_PATH)
            .unwrap_or_else(|_| panic!("{} not found.", MAINTENANCE_ACTION_PATH));

        if action == "clear-cache" {
            Self::clear_cache(&context);
        } else {
            println!("Unsupported maintenance action: {}", action);
        }

        Feedback::None
    }
}
