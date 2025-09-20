import socket
from datetime import datetime
import glob

SERIAL_PORT = 19837
SPECIAL_CTRL_CHAR = b"\x93"

REQ_ACTIVE = 1
REQ_SAVE_STATE = 2
REQ_LOAD_STATE = 3

SAVE_FMT = "%Y_%m_%d_%H_%M_%S.save"

print("=== BrightNES server ===")

while True:
    try:
        client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        client.connect(("localhost", SERIAL_PORT))
        break
    except ConnectionRefusedError:
        pass

print("[-] Established connection.")

def recv_all(num):
    buf = b""
    while len(buf) < num:
        chunk = client.recv(num - len(buf))
        buf += chunk
    return buf

while True:
    while client.recv(1) != SPECIAL_CTRL_CHAR:
        pass

    print("[-] Received special control character.")

    req = int.from_bytes(client.recv(1), "little")

    if req == REQ_ACTIVE:
        print("[-] Received: REQ_ACTIVE")

        client.send(b"\x01")
        print("[-] Sent active response.")
        continue
    elif req == REQ_SAVE_STATE:
        print("[-] Received: REQ_SAVE_STATE")

        num = int.from_bytes(recv_all(4), "little")
        cpu = recv_all(num)
        print(f"[-] Received CPU state. ({num} bytes)")

        num = int.from_bytes(recv_all(4), "little")
        ppu = recv_all(num)
        print(f"[-] Received PPU state. ({len(ppu)} bytes)")

        num = int.from_bytes(recv_all(4), "little")
        cartridge = recv_all(num)
        print(f"[-] Received Cartridge state. ({len(cartridge)} bytes)")

        now = datetime.now().strftime(SAVE_FMT)
        with open(f"saves/{now}", "wb") as f:
            f.write(len(cpu).to_bytes(4, "little"))
            f.write(cpu)
            f.write(len(ppu).to_bytes(4, "little"))
            f.write(ppu)
            f.write(len(cartridge).to_bytes(4, "little"))
            f.write(cartridge)

        print(f"[-] Saved state to saves/{now}.")
    elif req == REQ_LOAD_STATE:
        print("[-] Received: REQ_LOAD_STATE")

        latest_save = None
        latest_time = datetime.min
        for path in glob.glob("saves/*.save"):
            time = datetime.strptime(path[6:], SAVE_FMT)
            if time > latest_time:
                latest_time = time
                latest_save = path

        if latest_save is None:
            print("[-] No save file found.")
            continue

        print("[-] Loading state from", latest_save)

        with open(latest_save, "rb") as f:
            data = f.read()
            client.sendall(data)

        print("[-] Sent state.")
    else:
        print("[!] Unknown request:", req)
