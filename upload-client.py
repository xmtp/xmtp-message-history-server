
import hmac
import hashlib
import requests

# The shared secret key
secret_key = b"your-secret-key"

# The request payload
payload = b"your request payload"

# Compute the HMAC
hmac_instance = hmac.new(secret_key, payload, hashlib.sha256)
hmac_hex = hmac_instance.hexdigest()

# Send the request with the HMAC header
headers = {'X-HMAC': hmac_hex}
response = requests.post('http://0.0.0.0:5558/upload', headers=headers, data=payload)
