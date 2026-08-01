#!/bin/bash
set -e

echo "Setting up isolate cgroups..."

# 1. Mount/delegation of cgroups v2 controllers
if [ -f /sys/fs/cgroup/cgroup.subtree_control ]; then
    echo "Enabling cgroup controllers at root level..."
    echo "+cpu +memory +pids" > /sys/fs/cgroup/cgroup.subtree_control 2>/dev/null || true
fi

# Create cgroup root subdirectory for isolate
mkdir -p /sys/fs/cgroup/isolate

if [ -f /sys/fs/cgroup/isolate/cgroup.subtree_control ]; then
    echo "Enabling cgroup controllers at isolate level..."
    echo "+cpu +memory +pids" > /sys/fs/cgroup/isolate/cgroup.subtree_control 2>/dev/null || true
fi

# 2. Write custom isolate config
echo "Configuring isolate..."
mkdir -p /usr/local/etc
cat <<EOT > /usr/local/etc/isolate
box_root = /var/local/lib/isolate
lock_root = /run/isolate/locks
cg_root = /sys/fs/cgroup/isolate
first_uid = 60000
first_gid = 60000
num_boxes = 100
EOT

# Ensure required run directories exist
mkdir -p /var/local/lib/isolate
mkdir -p /run/isolate/locks

echo "Starting CodecSec server..."
exec /app/target/release/codec-sec
