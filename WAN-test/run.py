import paramiko
import toml
import utils
import random
import time
import datetime
import os

class Node:
    def __init__(self):
        self.peers = []
        self.id = -1
        self.address = None
        self.enode = None

class TestConfig:
    def __init__(self):
        self.test = False
        self.n = 0
        self.time_limit = 0
        self.start_nonce = 0
        self.payload = 0

class NodeRunner:
    def __init__(self, cfg_path):
        parsed = toml.load(cfg_path)
        self.geth_dir = parsed['bin']['geth_dir']
        self.nodes_dir = parsed['node']['dir']
        self.node_count = parsed['node']['count']
        self.sealer_count = parsed['node']['sealer_count']
        self.accounts_dir = parsed['run']['accounts_dir']
        self.hosts = parsed['run']['hosts']
        self.wan = parsed['run']['wan']
        if self.wan:
            assert len(self.hosts) >= self.node_count
        else:
            assert len(self.hosts) >= 1
        self.nodes = []
        self.connectors = []
        self.test_config = TestConfig()

        addresses = utils.load_addresses(self.accounts_dir)
        for i in range(len(addresses)):
            node = Node()
            node.id = i
            node.address = addresses[i]
            self.nodes.append(node)

        random_conn = False
        if parsed['node'].get('random_connect') is not None:
            random_conn = parsed['node'].get('random_connect')

        if random_conn:
            peer_count = parsed['node']['peer_count']
            for i in range(len(self.nodes)):
                pids = self.sample(peer_count, self.node_count, i)
                for pid in pids:
                    self.nodes[i].peers.append(self.nodes[pid])
        else:
            conn = parsed['node']['connection']
            for i in range(len(self.nodes)):
                pids = conn[i]
                for pid in pids:
                    assert pid != i
                    self.nodes[i].peers.append(self.nodes[pid])

        if parsed['test']['test']:
            self.test_config.test = True
            self.test_config.n = parsed['test']['n']
            self.test_config.time_limit = parsed['test']['period']
            self.test_config.start_nonce = parsed['test']['start_nonce']
            self.test_config.payload = parsed['test']['payload']

    def connect_nodes(self):
        for i in range(len(self.nodes)):
            node = self.nodes[i]
            if self.wan:
                host = self.hosts[i]
            else:
                host = self.hosts[0]
            connector = NodeConnector(
                hostname=host[0],
                port=int(host[1]),
                username=host[2],
                password=host[3],
                node=node
            )
            connector.connect()
            self.connectors.append(connector)

    def init_local_env(self):
        assert len(self.connectors) == len(self.nodes)
        for i in range(len(self.connectors)):
            self.connectors[i].init_local_env()

    def upload_data(self):
        assert len(self.connectors) == len(self.nodes)
        for i in range(len(self.connectors)):
            self.connectors[i].upload_data()

    def clear_data(self):
        assert len(self.connectors) == len(self.nodes)
        for i in range(len(self.connectors)):
            self.connectors[i].clear_data()

    def recover_data(self):
        assert len(self.connectors) == len(self.nodes)
        for i in range(len(self.connectors)):
            self.connectors[i].recover_data()

    def do_run_nodes(self):
        assert self.connectors is not None and len(self.connectors) > 0
        for i in range(len(self.nodes)):
            node = self.nodes[i]
            connector = self.connectors[i]
            connector.launch_geth()
            connector.console.send_with_resp('eth.accounts[0]')
            node.enode = connector.console.send_with_resp('admin.nodeInfo.enode')
            print(node.enode)
        self.connect_peers()
        self.start_mining()
        if self.test_config.test:
            self.send_txs()

    def close(self):
        for connector in self.connectors:
            connector.close()

    def send_txs(self):
        before = self.get_tx_cnt()
        print('Transaction counts before sending tx: ', before)
        ddl = datetime.datetime.now() + \
              datetime.timedelta(seconds=self.test_config.time_limit)
        start_nonce = self.test_config.start_nonce
        end_nonce = start_nonce + self.test_config.n
        for i in range(start_nonce, end_nonce):
            if datetime.datetime.now() > ddl:
                break
            for j in range(len(self.nodes)):
                self.send_tx(j, (j+1)%len(self.nodes), i, self.test_config.payload)
        time.sleep(
            (ddl - datetime.datetime.now()).seconds
        )
        after = self.get_tx_cnt()
        assert len(after) == len(before)
        print('Transaction counts after sending tx: ', after)
        dif = [after[i]-before[i] for i in range(len(after))]
        print('Transaction committed for each node: ', dif)
        print('Total committed transactions: ', sum(dif))

    def start_mining(self):
        for i in range(self.sealer_count):
            console = self.connectors[i].console
            console.send_with_resp('miner.start()')
            console.send_with_resp('clique.getSigners()')
            console.send_with_resp('eth.accounts[0]')
            console.send_with_resp('admin.peers')

    def connect_peers(self):
        for connector in self.connectors:
            for peer in connector.node.peers:
                connector.console.send_with_resp(
                    'admin.addPeer(\"{}\")'.format(peer.enode)
                )

    def get_tx_cnt(self):
        res = []
        for connector in self.connectors:
            cnt = int(
                connector.console.send_with_resp(
                    'eth.getTransactionCount(eth.accounts[0])')
            )
            res.append(cnt)
        return res

    def send_tx(self, x, y, nonce, payload):
        msg = 'eth.sendTransaction({{from:\"{}\", to:\"{}\", nonce: \"{}\", value:web3.toWei(1e+45, \"ether\"), data: \"0x{}\"}})'\
            .format(self.nodes[x].address, self.nodes[y].address, nonce, utils.randHex(payload))
        self.connectors[x].console.send_with_resp(msg)

    @staticmethod
    def sample(k, n, cur):
        assert k <= n
        pool = []
        for i in range(cur):
            pool.append(i)
        for i in range(cur+1, n):
            pool.append(i)
        for i in range(k, n-1):
            r = random.randint(0, i)
            if r < k:
                pool[r] = pool[i]
        return pool[:k]

class NodeConnector:
    def __init__(self,
                 hostname,
                 port,
                 username,
                 password,
                 node):
        self.hostname = hostname
        self.port = port
        self.username = username
        self.password = password
        self.node = node
        self.client = None
        self.console = None

    def connect(self):
        self.client = paramiko.SSHClient()
        self.client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
        try:
            self.client.connect(
                self.hostname, self.port, self.username, self.password
            )
        except Exception as e:
            print(e)
            self.client = None
            return

    def launch_geth(self):
        command = 'cd ./{}; ./geth2\
    --datadir "./nodes/node{}/data"\
    --networkid 666\
    --port={}\
    --ipcpath=geth.ipc\
    --unlock={}\
    --password=password\
    console 2>log{}'.format(self.node_path(), self.node.id, 4000+self.node.id, self.node.address, self.node.id)
        print('Launch Geth:', command)
        stdin, stdout, _ = self.client.exec_command(command)
        self.console = utils.Console(stdin, stdout)

    def node_path(self):
        return str(self.node.id)

    def init_local_env(self):
        assert self.client is not None
        self.client.exec_command('mkdir -p {}'.format(self.node_path()))
        ftp_client = self.client.open_sftp()
        try:
            ftp_client.chdir('./{}'.format(self.node_path()))
            ftp_client.put('./password', './password')
            ftp_client.put('./key.pem', './key.pem')
            ftp_client.put('./private.pem', './private.pem')
            ftp_client.put('./geth2', './geth2')
            ftp_client.chmod('./geth2', 0o777)
        except Exception:
            raise
        finally:
            ftp_client.close()

    def upload_data(self):
        assert self.client is not None
        id = self.node.id
        # does not check whether the local environment has been established or not
        print('Packing node{}...'.format(id))
        os.system('tar -cf ./nodes/node{}.tar ./nodes/node{}'.format(id, id))
        print('Pack complete')

        ftp_client = self.client.open_sftp()
        try:
            self.client.exec_command('mkdir -p {}'.format(self.node_path()))
            ftp_client.chdir('./{}'.format(self.node_path()))
            print('Uploading node{}...'.format(id))
            ftp_client.put('./nodes/node{}.tar'.format(id), './node{}.tar'.format(id))
            print('Upload complete')
        except Exception:
            raise
        finally:
            ftp_client.close()

    def recover_data(self):
        assert  self.client is not None
        id = self.node.id
        print('Unpacking node{}...'.format(id))
        command = 'tar -xf ./{}/node{}.tar -C ./{}'.format(self.node_path(), id, self.node_path())
        self.client.exec_command(command)

    def clear_data(self):
        assert self.client is not None
        id = self.node.id
        ftp_client = self.client.open_sftp()
        try:
            ftp_client.chdir('./{}'.format(self.node_path()))
            ftp_client.rename('./nodes/node{}'.format(id), './nodes/{}'.format(utils.randHex(16)))
        except Exception:
            raise
        finally:
            ftp_client.close()

    def close(self):
        if self.client is not None:
            if self.console is not None:
                self.console.send('exit')
                status = self.console.reader.channel.recv_exit_status()
                if status != 0:
                    print('Warn: node {} exits abnormally.'.format(self.node.id))
            self.client.close()
            self.client = None
            self.console = None

def main():
    runner = NodeRunner('./config.toml')
    runner.connect_nodes()
    runner.init_local_env()
    runner.upload_data()
    runner.clear_data()
    runner.recover_data()
    runner.do_run_nodes()
    runner.close()

if __name__ == '__main__':
    main()
