#!/bin/bash

# Kill background jobs on exit
trap "kill 0" EXIT

echo "--- Distributed HPC Demo ---"

# 1. Start the Hub
echo "[1/4] Starting Hub on port 8080..."
cargo run -p hpc_node -- hub --port 8080 > hub.log 2>&1 &
HUB_PID=$!
sleep 3

# 2. Start the Relay
echo "[2/4] Starting Relay on port 8081..."
cargo run -p hpc_node -- relay --port 8081 --hub-url http://localhost:8080 > relay.log 2>&1 &
RELAY_PID=$!
sleep 3

# 3. Submit a Job
echo "[3/4] Submitting a job to the Hub..."
cargo run -p hpc_node -- submit --hub-url http://localhost:8080

echo "[4/4] Waiting for Relay to process..."
sleep 5

echo "--- Demo Logs ---"
echo ">> HUB LOG (last 5 lines):"
tail -n 5 hub.log
echo ""
echo ">> RELAY LOG (last 5 lines):"
tail -n 5 relay.log

echo "--- Done ---"

