#!/bin/bash
set -euo pipefail

curl -X POST http://localhost:8000/api/$1 -d '{ "content":"'"$2"'"}' -H "Content-Type: application/json"
