#!/usr/bin/env python3
"""
AORUI Agent Client SDK (Python)
================================
Enables any external script or AI agent framework (LangChain, AutoGen, CrewAI,
OpenAI Assistants, Claude, Antigravity) to drive an AORUI desktop application
in real time via HTTP/JSON-RPC.
"""

import sys
import argparse
import requests
import json

class AoruiClient:
    """Client for controlling an AORUI application via its Agent HTTP Bridge."""

    def __init__(self, host: str = "127.0.0.1", port: int = 8765, token: str = None):
        self.base_url = f"http://{host}:{port}"
        self.headers = {"Content-Type": "application/json; charset=utf-8"}
        if token:
            self.headers["Authorization"] = f"Bearer {token}"

    def health(self) -> dict:
        """Queries health status, active port, and memory sandbox mode."""
        resp = requests.get(f"{self.base_url}/health", headers=self.headers, timeout=3)
        resp.raise_for_status()
        return resp.json()

    def prompt(self, text: str) -> dict:
        """Dispatches a natural language instruction or command to the cognitive loop."""
        resp = requests.post(
            f"{self.base_url}/prompt",
            headers=self.headers,
            json={"prompt": text},
            timeout=10
        )
        resp.raise_for_status()
        return resp.json()

    def interrupt(self) -> dict:
        """Immediately interrupts any in-progress agent tool execution."""
        resp = requests.post(f"{self.base_url}/interrupt", headers=self.headers, timeout=3)
        resp.raise_for_status()
        return resp.json()

def main():
    parser = argparse.ArgumentParser(description="AORUI Agent Remote Controller CLI")
    parser.add_argument("--prompt", "-p", type=str, help="Instruction or action to execute")
    parser.add_argument("--health", action="store_true", help="Check bridge connectivity and sandbox mode")
    parser.add_argument("--port", type=int, default=8765, help="Application port (default: 8765)")
    parser.add_argument("--token", type=str, default=None, help="Optional Bearer token for authentication")

    args = parser.parse_args()
    client = AoruiClient(port=args.port, token=args.token)

    if args.health:
        res = client.health()
        print(json.dumps(res, indent=2, ensure_ascii=False))
    elif args.prompt:
        res = client.prompt(args.prompt)
        print(json.dumps(res, indent=2, ensure_ascii=False))
    else:
        print("Usage: python aorui_client.py --prompt 'Purge caches and refresh telemetry' or --health")

if __name__ == "__main__":
    main()
