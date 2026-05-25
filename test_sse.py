import requests
import sys
import os
import threading
import time

api = os.environ['API']
token = os.environ['TOKEN']
headers = {'Authorization': f'Bearer {token}'}

def listen_events():
    print("Connecting to /events...")
    r = requests.get(f"http://{api}/events", headers=headers, stream=True)
    for line in r.iter_lines():
        if line:
            print("EVENT:", line.decode())

def actions():
    time.sleep(1)
    print("Subscribing to 'test-topic'...")
    requests.post(f"http://{api}/subscribe", headers=headers, json={"topic": "test-topic"})
    time.sleep(1)
    print("Publishing to 'test-topic'...")
    import base64
    payload = base64.b64encode(b"hello").decode()
    requests.post(f"http://{api}/publish", headers=headers, json={"topic": "test-topic", "payload": payload})

t1 = threading.Thread(target=listen_events, daemon=True)
t2 = threading.Thread(target=actions)

t1.start()
t2.start()

time.sleep(4)
print("Done.")
