import hmac
import hashlib
import requests
import os

file_path = "test_bundle.txt"

secret_key = os.environ.get("SECRET_KEY", "").encode()

# Ensure the secret key is not empty
if not secret_key:
    print("SECRET_KEY environment variable is not set.")
    exit(1)

# The request payload consisting of a message history bundle
with open(file_path, 'rb') as file:
    file_content = file.read()
    print(file_content)

    # Compute the HMAC
    hmac_instance = hmac.new(secret_key, file_content, hashlib.sha256)
    hmac_hex = hmac_instance.hexdigest()
    print(hmac_hex)

    # Send the request with the HMAC header
    headers = {'X-HMAC': hmac_hex}
    response = requests.post('http://0.0.0.0:5558/upload', headers=headers, data=file_content)

# Log the response
print(f"Response Status Code: {response.status_code}")
print(f"Response Body: {response.text}")
