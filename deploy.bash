DEPLOY_HOST=pi@192.168.4.119
BINARY_NAME=rpi_led
TARGET_ARCH=arm-unknown-linux-gnueabihf

cross build --target=$TARGET_ARCH --release
scp target/$TARGET_ARCH/release/$BINARY_NAME $DEPLOY_HOST:
ssh $DEPLOY_HOST "chmod +x $BINARY_NAME"
