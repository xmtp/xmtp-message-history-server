import requests
import os

def download_messages_bundle(bundle_id, hmac_value):
    """
    Downloads a file from the specified endpoint using a GET request with an X-HMAC header.

    Parameters:
    - bundle_id (str): The unique identifier for the file.
    - hmac_value (str): The HMAC signature value of the file with `bundle_id`.
    """
    
    # Construct the URL with the file's BUNDLE_ID
    url = f"http://0.0.0.0:5558/files/{bundle_id}"

    # Send the GET request with the X-HMAC header
    headers = {'X-HMAC': hmac_value}
    response = requests.get(url, headers=headers)

    if response.status_code == 200:
        print("File downloaded successfully.")
        # Here, you will want to save the file content to a file, for example:
        with open(f"messages_bundle_{bundle_id}", 'wb') as file:
            file.write(response.content)
    else:
        print(f"Failed to download file. Status code: {response.status_code} Response: {response.text}")


if __name__ == "__main__":
    # The assigned bundle_id returned from calling `python upload-bundle.py`
    bundle_id = os.environ.get("BUNDLE_ID", "").encode()
    # The value from calculating the hmac signature for the uploaded messages bundle file
    hmac_value = os.environ.get("HMAC_VALUE", "").encode()  

    if not bundle_id:
        print("BUNDLE_ID environment variable is not set.")
        exit(1)
        
    if not hmac_value:
        print("HMAC_VALUE environment variable is not set.")
        exit(1)
    
    download_messages_bundle(bundle_id, hmac_value)
