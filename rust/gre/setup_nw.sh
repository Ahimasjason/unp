#!/bin/bash
set -ex
# create thge namespace
ip netns add ns_a
ip netns add ns_b


# create the veth pair 
ip link add  veth_a type veth peer name veth_b
# like the ns_a to veth_a
ip link set veth_a netns ns_a
ip link set veth_b netns ns_b

# up the loopback in the ns_a
ip netns exec ns_a ip link set lo up
ip netns exec ns_a ip addr add 192.168.50.1/24 dev veth_a
ip netns exec ns_a ip link set veth_a up



ip netns exec ns_b ip link set lo up
ip netns exec ns_b ip addr add 192.168.50.2/24 dev veth_b
ip netns exec ns_b ip link set veth_b up


