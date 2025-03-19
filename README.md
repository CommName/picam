# PI-Cam
*Note*: PI-Cam is in early development and everything is still experimental

PI-Cam is a minimilistic, lightweight, all you need software for streaming and recording web cameras. It is designed to run on any Raspberry Pi device.

Key features:
- Live Streaming and Recording: The software allows real-time low latency video streaming from cameras, and can store recorded footage locally.
- Remote Access: The software often supports remote viewing of camera feeds via a web browser, enabling users to monitor their premises from anywhere
- Low Resource Requirements: The software is lightweight, designed to run smoothly on the Raspberry Pi's limited processing power and memory.

# Build 

## Build for Pi Zero W

### Prerequirments
1. Install [QEMU](https://www.stereolabs.com/docs/docker/building-arm-container-on-x86)
1. Build docker image builder
```
docker image build --platform arm  --file=./Dockerbuilder.arm -t pibuilder:arm .
```

### Build
Currently build process is manual from inside of docker image.
Run docker builder container and all tools are availbe  

If you are getting error and you can't pull crates create a tmpfs
```
sudo mount -t tmpfs -o size=10G tmpfs /tmp/cargo
```

Run container
```
docker run --privileged --platform arm --rm -it  --network host  --mount type=bind,src=./,dst=/app -v /tmp/cargo:/root/.cargo/registry/index/  pibuilder:arm /bin/bash
```
All code is in `/app` directory
```
cd /app
```
Create a binary 
```
cargo build --release
```

Copy binary and entire frontend folder to pi zero. Make sure that they are in the same folder.
Configure systemd service to run automatically. Example of configuration can be found `./debian/service`