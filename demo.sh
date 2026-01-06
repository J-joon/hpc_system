#!/bin/bash

# Kill background jobs on exit
trap "kill 0" EXIT

echo "--- Distributed HPC Demo (Advanced) ---"

# 1. Start the Hub
echo "[1/7] Starting Hub on port 8080..."
cargo run -p hpc_node -- hub --port 8080 > hub.log 2>&1 &
HUB_PID=$!
sleep 3

# 2. Start the Batch Relay
echo "[2/7] Starting Batch Relay on port 8081..."
cargo run -p hpc_node -- relay --port 8081 --ns batch --hub-url http://localhost:8080 > relay_batch.log 2>&1 &
sleep 2

# 3. Start the Simulation Relay
echo "[3/7] Starting Simulation Relay on port 8082..."
cargo run -p hpc_node -- relay --port 8082 --ns simulation --hub-url http://localhost:8080 > relay_sim.log 2>&1 &
sleep 2

# 4. Start the Resource Monitor
echo "[4/7] Starting Resource Monitor..."
cargo run -p hpc_node -- monitor --hub-url http://localhost:8080 --node-id demo-node --interval 2 > monitor.log 2>&1 &
sleep 2

# 5. Submit a Batch Job
echo "[5/7] Submitting a Batch job..."
cargo run -p hpc_node -- submit --ns batch --hub-url http://localhost:8080

# 6. Submit a Simulation Job
echo "[6/7] Submitting a Simulation job..."
cargo run -p hpc_node -- submit --ns simulation --hub-url http://localhost:8080

echo "[7/7] Waiting for lifecycle events (approx 10s)..."
sleep 10

echo "--- Demo Logs ---"
echo ">> HUB LOG (last 10 lines):"
tail -n 10 hub.log
echo ""
echo ">> BATCH RELAY LOG (last 5 lines):"
tail -n 5 relay_batch.log
echo ""
echo ">> SIMULATION RELAY LOG (last 10 lines):"
tail -n 10 relay_sim.log
echo ""
echo ">> MONITOR LOG (last 3 lines):"
tail -n 3 monitor.log

echo "--- Done ---"
