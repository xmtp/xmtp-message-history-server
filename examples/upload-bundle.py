import hmac
import hashlib
import pyAesCrypt
import requests
import os

file_path = "test_bundle.jsonl"

def encrypt(source, key):
    key_str = key.decode('utf-8')
    output = source + ".aes"
    pyAesCrypt.encryptFile(source, output, key_str)
    return output

def upload_message_bundle(file_path, signing_key):
    """
    Uploads a file to the specified endpoint. The file should be encrypted.

    Parameters:
    - file_path (str): The path to the file to upload.
    - signing_key (str): The key used to sign the HMAC.
    """
     # The request payload consisting of a message history bundle
    with open(file_path, 'rb') as file:
        file_content = file.read()

        # Compute the HMAC
        hmac_instance = hmac.new(signing_key, file_content, hashlib.sha256)
        hmac_hex = hmac_instance.hexdigest()
        print(f"HMAC: {hmac_hex}")

        # Send the request with the HMAC header
        headers = {'X-HMAC': hmac_hex}
        response = requests.post('http://0.0.0.0:5558/upload', headers=headers, data=file_content)
        # Log the response
        print(f"Response Status Code: {response.status_code}")
        print(f"Response Body: {response.text}")

if __name__ == "__main__":
    # Ensure the signing key is not empty
    signing_key = os.environ.get("SIGNING_KEY", "").encode()
    if not signing_key:
        print("SIGNING_KEY environment variable is not set.")
        exit(1)

    # Ensure the aes key is not empty
    aes_key = os.environ.get("AES_KEY", "").encode()
    if not aes_key:
        print("AES_KEY environment variable is not set.")
        exit(1)
        
    encrypted_file = encrypt(file_path, aes_key)
    
    upload_message_bundle(encrypted_file, signing_key)
    

