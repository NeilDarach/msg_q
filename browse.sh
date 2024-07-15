#!/bin/bash
set -euo pipefail

if [[ $# -ge 2 ]] ; then
  mid="&mid=${2}"
else
  mid=""
fi


curl "http://localhost:8000/api/$1?action=browse${mid}" 
