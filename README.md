# ImageQuick

> An image importer for photographers with too mnay photos and too little time.

The goal of this ImageQuick is to make it easy to filter through thousands of images from your SD card. ImageQuick also supports image uploading to ImageThing.

# ImageThing

ImageThing is a website where you can share your photography work. It integrates with ImageQuick and is backed by Cloudflare R2, making image storage for thousands of high-quality images fast and cheap.

ImageThing works with ImageQuick, so if you're taking lots of photos on the go and need a quick backup that you can share with friends, ImageThing is your best friend.

## Performance

This whole project is written in Rust and uses all the speedy techniques:
- multithreaded FS
- zune-jpeg
- cloudflare r2 for thumbnail generation and storage




