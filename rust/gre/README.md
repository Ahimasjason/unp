

```bash
cargo build --offline
sudo bash setup_nw.sh
sudo ip netns exec ns_a ping -c 3 192.168.50.2
```

```bash
# Terminal A
sudo ip netns exec ns_a ./target/debug/gre tun0 10.0.0.1 255.255.255.0 192.168.50.2
```

```bash
# Terminal B
sudo ip netns exec ns_b ./target/debug/gre tun0 10.0.0.2 255.255.255.0 192.168.50.1
```


```bash
sudo ip netns exec ns_a ping -c 4 10.0.0.2
```

```bash
sudo ip netns exec ns_a tcpdump -ni veth_a -vv 'ip proto 47'
```


```bash
sudo ip netns exec ns_a sh -c \
    "tcpdump -nn -l -i veth_a -vv 'ip proto 47' 2>&1 | cat"

```