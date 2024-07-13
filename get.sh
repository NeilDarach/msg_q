#!/bin/bash
set -euo pipefail

curl "http://localhost:8000/api/get/$1/$2" 
