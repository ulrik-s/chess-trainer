#!/bin/sh
# Arkitekturregel: domänlagret får inte ha några beroenden alls.
# Lagerordningen domain -> app -> adapters bevakas av Cargo självt (en crate kan bara
# bero på det den listar), så det enda som behöver kontrolleras är domänets tomma lista.
set -eu
deps=$(cargo tree -p chess-trainer-domain -e normal --prefix none --locked | tail -n +2)
if [ -n "$deps" ]; then
  echo "chess-trainer-domain får inte ha beroenden, men har:" >&2
  echo "$deps" >&2
  exit 1
fi
echo "arch: ok (domain har inga beroenden)"
