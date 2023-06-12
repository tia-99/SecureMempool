use std::path::{Path, PathBuf};
use std::io::prelude::*;
use std::str::FromStr;
use std::error::Error;
use std::fmt;
use std::process::{Command, Stdio};
use std::env;

use crate::utils::{self, Console, ConsoleInteractor, node_dir};
use crate::{Address, NETWORK, NETWORK_ID};

struct InitError;

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Debug for InitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for InitError {}

#[derive(Debug)]
pub struct NodeInitializer {
    geth_dir:       PathBuf,
    puppeth_dir:    PathBuf,
    nodes_dir:       PathBuf,
    node_count:     usize,
    sealer_count:   usize,
    out:            PathBuf,
    mining_period:  u32,
}

impl NodeInitializer {
    pub fn new_with_cfg_file(path: &Path) -> NodeInitializer {
        let parsed = utils::read_toml(path);
        NodeInitializer {
            geth_dir:       PathBuf::from_str(parsed["bin"]["geth_dir"].as_str().unwrap()).unwrap(),
            puppeth_dir:    PathBuf::from_str(parsed["bin"]["puppeth_dir"].as_str().unwrap()).unwrap(),
            nodes_dir:      PathBuf::from_str(parsed["node"]["dir"].as_str().unwrap()).unwrap(),
            node_count:     parsed["node"]["count"].as_integer().unwrap() as usize,
            sealer_count:   parsed["node"]["sealer_count"].as_integer().unwrap() as usize,
            out:            PathBuf::from_str(parsed["init"]["accounts_dir"].as_str().unwrap()).unwrap(),
            mining_period:  if let Some (mining_period) = parsed["init"].get("mining_period") { mining_period.as_integer().unwrap() as u32 } else { 15 },

        }
    }

    pub fn do_init_node(&self) {
        let accounts = self.create_accounts();
        self.create_genesis(&accounts);
        self.init_nodes();
    }

    fn init_nodes(&self) {
        let mut genesis_dir = self.nodes_dir.clone();
        genesis_dir.push(Path::new(&format!("{}.json", NETWORK)));
        let genesis_dir = genesis_dir.into_os_string().into_string().unwrap();

        for i in 0..self.node_count {
            self.init_node(i, &genesis_dir);
        }
    }

    fn init_node(&self, id: usize, genesis_dir: &str) {
        let mut geth = Command::new(&self.geth_dir)
            .arg(format!("--datadir={}", node_dir(&self.nodes_dir, id)))
            .arg("init")
            .arg(genesis_dir)
            .spawn()
            .unwrap();
        geth.wait().unwrap();
    }

    // assumes self.node_count >= self.sealer_count
    fn create_genesis(&self, accounts: &Vec<Address>) {
        let mut dir = env::current_dir().unwrap();
        dir.push(Path::new(".puppeth"));
        let exist = dir.is_dir();

        println!("***puppeth_dir: {}***", self.puppeth_dir.to_string_lossy());
        let mut puppeth = Command::new(&self.puppeth_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        
        let console = Console
            ::<utils::ChildReader, utils::ChildWriter>
            ::from_child(&mut puppeth, "PUPPETH");
        
        let mut itr = ConsoleInteractor::new(console);
        itr.send_on_prompt(NETWORK.as_bytes());

        if exist {
            println!("Target network exists, rewriting...");
            itr.send_on_prompt(b"2");
            itr.send_on_prompt(b"3");
        }
        itr.send_on_prompt(b"2");
        itr.send_on_prompt(b"1");
        itr.send_on_prompt(b"2");
        itr.send_on_prompt(u32::to_string(&self.mining_period).as_bytes());

        for i in 0..self.sealer_count {
            itr.send_on_prompt(accounts[i].as_bytes());
        }
        itr.send_on_prompt(b"");

        for i in 0..self.node_count {
            itr.send_on_prompt(accounts[i].as_bytes());
        }
        itr.send_on_prompt(b"");

        itr.send_on_prompt(b"");
        itr.send_on_prompt(u64::to_string(&NETWORK_ID).as_bytes());

        itr.send_on_prompt(b"2");
        itr.send_on_prompt(b"2");

        let genesis_path = self.nodes_dir.clone().into_os_string().into_string().unwrap();
        itr.send_on_prompt(genesis_path.as_bytes());

        itr.send_on_prompt(b"");

        puppeth.kill().unwrap();
    }

    fn create_accounts(&self) -> Vec<Address> {
        let mut accounts = vec![];
        for i in 0..self.node_count {
            let account = self.create_account(i);
            accounts.push(account);
        }
        utils::save_addrs(accounts.clone(), &self.out).unwrap();

        accounts
    }

    fn create_account(&self, id: usize) -> Address {
        let mut geth = Command::new(&self.geth_dir)
            .arg(format!("--datadir={}", node_dir(&self.nodes_dir, id)))
            .arg("account")
            .arg("new")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut geth_in = geth.stdin.take().unwrap();
        let mut geth_out = geth.stdout.take().unwrap();
        geth_in.write_all(b"\n\n").unwrap();
        let mut res = String::new();
        geth_out.read_to_string(&mut res).unwrap();
        let idx = res.find("0x").unwrap() + 2;
        res[idx..(idx+40)].to_string()
    }
}