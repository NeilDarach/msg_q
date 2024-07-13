#!/bin/bash
set -euo pipefail

curl -X POST http://localhost:8000/api/create -d '{ "queue_name": "'$1'", "content":"'$2'"}' -H "Content-Type: application/json"
