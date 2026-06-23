import urllib.request
import json

data = {
    "raw_payload": "Hello world maina ke che aki memeo veotr caht inut aout save brain me check",
    "vector_data": [0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,0.1,0.11,0.12,0.13,0.14,0.15,0.16],
    "model_creator_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "payload_type": "text"
}

req = urllib.request.Request("http://localhost:9090/neuron", data=json.dumps(data).encode('utf-8'), headers={'Content-Type': 'application/json', 'x-tenant-id': 'default_sandbox'}, method='POST')
try:
    response = urllib.request.urlopen(req)
    res_data = json.loads(response.read())
    print(f"CREATED NEURON: {res_data}")
    
    neuron_id = res_data['neuron_id']
    req_get = urllib.request.Request(f"http://localhost:9090/neuron/{neuron_id}", headers={'x-tenant-id': 'default_sandbox'}, method='GET')
    response_get = urllib.request.urlopen(req_get)
    get_data = json.loads(response_get.read())
    
    print("\nFETCHED FROM DB:")
    raw_payload_bytes = get_data['raw_payload']
    text = ''.join(chr(b) for b in raw_payload_bytes)
    print(f"Text in DB: {text}")
    print(f"Vectors in DB: {get_data['vector_data']}")
    
except Exception as e:
    print(f"Error: {e}")
