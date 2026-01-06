#!/bin/bash
PORT=$(shuf -i 5000-6000 -n 1)
echo "Simulation Env started on port $PORT"
echo $PORT > .env_port
sleep 5
echo "Simulation Env finished"
