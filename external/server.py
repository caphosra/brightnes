import socket

SERIAL_PORT = 19837
SPECIAL_CTRL_CHAR = b"\x93"

print("=== BrightNES server ===")

while True:
    try:
        client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        client.connect(("localhost", SERIAL_PORT))
        break
    except ConnectionRefusedError:
        pass

print("[-] Established connection.")

while True:
    while client.recv(1) != SPECIAL_CTRL_CHAR:
        pass

    print("[-] Received special control character.")

    num = int.from_bytes(client.recv(4), "little")
    print(f"[-] Going to receive {num} bytes.")

    data = client.recv(num)
    print(data.decode("utf-8"))
