import toml
import random
import string

def randHex(N):
    return ''.join(random.choice(string.ascii_lowercase+string.digits) for _ in range(N))

def load_addresses(address_dir):
    loaded = toml.load(address_dir)
    return loaded['addrs']

class Console:
    def __init__(self, writer, reader):
        self.reader = reader
        self.writer = writer
        self.delimiter = b'>'
        self.recv()

    def send(self, msg):
        self.writer.write(msg)
        self.writer.write(b'\n')

    def recv(self):
        chars = []
        while True:
            cur = self.reader.read(1)
            chars.append(cur.decode())
            if cur == self.delimiter:
                break
        return ''.join(chars)
    
    def send_with_resp(self, msg):
        self.log('Send to console: {}'.format(msg))
        self.send(msg)
        received = self.recv()
        self.log('Receive from console: {}'.format(received))
        return received.strip('> \n\t\"')

    def log(self, s):
        print(s)
