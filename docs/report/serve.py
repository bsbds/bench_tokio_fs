# import http.server
# import socketserver

# PORT = 8000

# Handler = http.server.SimpleHTTPRequestHandler

# with socketserver.TCPServer(("", PORT), Handler) as httpd:
#     print("serving at port", PORT)
#     httpd.serve_forever()

import http.server
import socketserver
import os

class MyHttpRequestHandler(http.server.SimpleHTTPRequestHandler):
    def translate_path(self, path):
        # Get the base directory of the script
        base_dir = os.path.dirname(os.path.abspath(__file__))
        # Allow access to the parent directory
        parent_dir = os.path.abspath(os.path.join(base_dir, os.pardir))
        # Construct the full path
        full_path = os.path.join(parent_dir, path.lstrip('/'))
        return full_path

# Define the handler to use
handler = MyHttpRequestHandler

# Define the server address and port
PORT = 8001
server_address = ("", PORT)

# Create the HTTP server
httpd = socketserver.TCPServer(server_address, handler)

print(f"Serving HTTP on port {PORT}")

# Serve until interrupted
httpd.serve_forever()
