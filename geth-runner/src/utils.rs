use std::io::{self, Read, BufRead, Write};
use std::fmt;
use std::format_args;
use std::process;
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use toml::Value;
use rand::prelude::*;

use crate::Address;

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
struct Accounts {
    addrs: Vec<Address>,
}

pub fn rand_hex(len: usize) -> String {
    let mut rng = rand::thread_rng();
    let dg = "0123456789abcdef".as_bytes();
    String::from_utf8(
        (0..len).map(|_| dg[rng.gen_range(0..dg.len())]).collect()
    ).unwrap()
}

pub fn save_addrs(addrs: Vec<Address>, path: &Path) -> io::Result<()> {
    let contents = toml::to_string(
        &Accounts {
            addrs,
        }
    ).unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

pub fn load_addrs(path: &Path) -> io::Result<Vec<Address>> {
    let mut contents = String::new();
    let mut file = File::open(path)?;
    file.read_to_string(&mut contents)?;
    let accounts: Accounts = toml::from_str(&contents).unwrap();
    Ok(accounts.addrs)
}

pub fn node_dir(nodes_dir: &PathBuf, id: usize) -> String {
    let mut nodes_dir = nodes_dir.clone();
    let mut subdir = format!("node{}/data", id);
    nodes_dir.push(Path::new(&mut subdir));
    nodes_dir.into_os_string().into_string().unwrap()
}

pub fn read_toml(path: &Path) -> Value {
    let mut file = File::open(&path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    contents.parse::<Value>().unwrap()
}

pub struct Console<T, U>
    where T: Read + BufRead, U: Write
{
    reader: T,
    writer: U,
    name:   String,
}

pub type ChildReader = io::BufReader<process::ChildStdout>;
pub type ChildWriter = process::ChildStdin;
pub type ConsoleFromChild = Console<ChildReader, ChildWriter>;

impl<T, U> Console<T, U>
    where T: Read + BufRead, U: Write
{
    // consumes the stdout and stdin of child,
    // no guarantee on the termination of child
    pub fn from_child(child: &mut process::Child, name: &str) -> ConsoleFromChild {
        Console {
            reader: io::BufReader::new(child.stdout.take().unwrap()),
            writer: child.stdin.take().unwrap(),
            name:   String::from(name),
        }
    }
}

pub struct ConsoleInteractor<T, U>
    where T: Read + BufRead, U: Write
{
    console:    Console<T, U>,
    delimeter:  u8,
}

impl<T, U> ConsoleInteractor<T, U>
where T: Read + BufRead, U: Write {
    pub fn new(console: Console<T, U>) -> ConsoleInteractor<T, U> {
        ConsoleInteractor {
            console,
            delimeter: b'>',
        }
    }

    pub fn _delimeter(&self) -> u8 {
        self.delimeter
    }

    pub fn _set_delimeter(&mut self, delimeter: u8) {
        self.delimeter = delimeter;
    }

    pub fn send(&mut self, msg: &[u8]) -> io::Result<()> {
        self.console.writer.write_all(msg)?;
        self.console.writer.write_all(b"\n")
    }

    pub fn recv(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let n = self.console.reader.read_until(self.delimeter, buf)?;
        buf.pop();
        Ok(n-1)
    }

    pub fn send_on_prompt(&mut self, msg: &[u8]) {
        let mut buf = Vec::new();
        self.recv(&mut buf).expect("Read prompt failed");
        self.log(format_args!("receive from console: {}", String::from_utf8(buf).unwrap()));
        self.log(format_args!("send to console: {}", String::from_utf8_lossy(msg)));
        self.send(msg).expect("Send message failed");
    }

    pub fn send_with_resp(&mut self, msg: &[u8]) -> String {
        self.log(format_args!("send to console: {}", String::from_utf8_lossy(msg)));
        self.send(msg).expect("Send message failed");
        let mut buf = Vec::new();
        self.recv(&mut buf).expect("Read prompt failed");
        self.log(format_args!("receive from console: {}", String::from_utf8(buf.clone()).unwrap()));
        
        let resp = String::from_utf8(buf).expect("Received invalid utf-8 string from console");
        let resp = String::from(resp.trim_matches(|ch: char| ch.is_whitespace() || ch == '\"'));

        resp
    }

    fn log(&self, args: fmt::Arguments) {
        print!("Console {}: ", self.console.name);
        println!("{}", args);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_rand_hex() {
        let test_len = 16;
        let cnt = 10;
        for len in 1..test_len {
            for _ in 0..cnt {
                println!("{}", super::rand_hex(len));
            }
        }
    }
}