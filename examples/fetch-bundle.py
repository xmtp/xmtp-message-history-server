import pyAesCrypt
import requests
import os

def decrypt(source, key):
    parts = source.rsplit(".aes", 1)
    output = parts[0] if len(parts) > 1 else source
    key_str = key.decode('utf-8')
    pyAesCrypt.decryptFile(source, output, key_str)
    return

def download_messages_bundle(bundle_id, hmac_value, signing_key, aes_key):
    """
    Downloads a file from the specified endpoint using a GET request with X-HMAC & X-SIGNING-KEY headers.

    Parameters:
    - bundle_id (str): The unique identifier for the file.
    - hmac_value (str): The HMAC signature value of the file with `bundle_id`.
    - signing_key (str): The key used to sign the bundle.
    - aes_key (str): The AES key used to decrypt the downloaded file.
    """
    
    # Construct the URL with the file's BUNDLE_ID
    bundle_id_str = bundle_id.decode('utf-8')
    url = f"http://0.0.0.0:5558/files/{bundle_id_str}"

    # Send the GET request with the X-HMAC header
    hmac_value_str = hmac_value.decode('utf-8')
    headers = {'X-HMAC': hmac_value_str, 'X-SIGNING-KEY': signing_key}
    response = requests.get(url, headers=headers)

    if response.status_code == 200:
        print("File downloaded successfully.")
        # Here, you will want to save the file content to a file.
        file_name = f"messages_bundle_{bundle_id_str}.aes"
        with open(file_name, 'wb') as file:
            file.write(response.content)
        
        decrypt(file_name, aes_key)
        print(f"Successfully decrypted {file_name}")
            
    else:
        print(f"Failed to download file. Status code: {response.status_code} Response: {response.text}")


if __name__ == "__main__":
    # The assigned bundle_id returned from calling `python upload-bundle.py`
    bundle_id = os.environ.get("BUNDLE_ID", "").encode()
    if not bundle_id:
        print("BUNDLE_ID environment variable is not set.")
        exit(1)
        
    # The value from calculating the hmac signature for the uploaded messages bundle file
    hmac_value = os.environ.get("HMAC_VALUE", "").encode()  
    if not hmac_value:
        print("HMAC_VALUE environment variable is not set.")
        exit(1)
        
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

    download_messages_bundle(bundle_id, hmac_value, signing_key, aes_key)
