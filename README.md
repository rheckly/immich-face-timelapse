# Immich Face-Timelapse

Based on [ArnaudCrl/immich-automated-selfie-timelapse](https://github.com/ArnaudCrl/immich-automated-selfie-timelapse). Technical names (repository, Docker image, container) are unchanged.

***Generate timelapse videos of your loved ones' portraits from your [Immich](https://immich.app/) photo library.***

Use Immich's built-in face recognition to find all photos of a person, then runs them through a configurable processing pipeline that crops, aligns, filters, and compiles the results into a smooth timelapse video.

<p align="center">
  <img src="resources/main_view.png" alt="Main View">
</p>

<p align="center">
  <img src="resources/result.gif" alt="Timelapse example" width="40%">
</p>

## Features

- **Face detection & cropping** - Uses Immich's face recognition metadata to locate and crop faces automatically
- **Face alignment** - Aligns all photos based on eye positions for a stable, smooth timelapse
- **Head pose filtering** - Skips non-front-facing photos using an ONNX deep learning model (DMHead)
- **Blink detection** - Filters out photos where eyes are closed using Eye Aspect Ratio analysis
- **Blur detection** - Discards blurry photos using gradient magnitude analysis (Sobel operator)
- **Brightness filtering** - Removes photos that are too dark or overexposed
- **Face resolution filtering** - Skips photos where the face is too small
- **Photo limiting** - Caps photos per day/week/month to avoid over-representing busy periods
- **Timestamp overlay** - Optionally overlays the date on each frame
- **Video compilation** - Automatically compiles processed photos into an MP4 timelapse using FFmpeg
- **Web UI** - Configure all settings and monitor progress through a built-in web interface

<p align="center">
  <img src="resources/processing_detail.png" alt="Processing">
</p>

## Quick Start

### Immich API key

To access your immich library, this project requires an Immich API key.  
Follow this guide to create one: https://immich.app/docs/features/command-line-interface#obtain-the-api-key

Here are the features that need to be enabled:
- `album.read`
- `asset.download`
- `asset.read`
- `asset.view`
- `face.read`
- `person.read`
- `server.about`

*Note: It is advised to store this API key in a `.env` file adjacent to your `docker-compose.yml` rather than in plain text.*

### Docker Compose (recommended)

Feel free to deploy this container on a more powerfull PC for faster timelapse generation. This is a 'one shot' application, it does not necessarily need to be deployed on a always on server.

```yaml
services:
  immich-selfie-timelapse:
    image: arnaudcayrol/immich-selfie-timelapse
    container_name: immich-selfie-timelapse
    user: 1000:1000
    ports:
      - "5000:5000"
    environment:
      - IMMICH_API_KEY=abcdefghijklmnopqrstuvwxyz
      - IMMICH_BASE_URL=http://your-immich-host:2283
      # optional: "Shared with" source from Immich Smart Albums
      # - SMARTALBUMS_URL=http://your-smartalbums-host:8080
      # - CONNECT_KEY=${CONNECT_KEY}
    volumes:
      - ./config:/app/config
      - ./output:/app/output
      - ./music:/app/music:ro   # optional: background music
    restart: unless-stopped
```

### Docker Run

```bash
docker run -d \
  --name immich-selfie-timelapse \
  -p 5000:5000 \
  -e IMMICH_API_KEY=your-api-key-here \
  -e IMMICH_BASE_URL=http://your_server:2283 \
  -e SMARTALBUMS_URL=http://your_server:8080 \
  -e CONNECT_KEY=your-connect-key \
  -v ./config:/app/config \
  -v ./output:/app/output \
  -v ./music:/app/music:ro \
  arnaudcayrol/immich-selfie-timelapse
```

Then open **http://your_server:5000** to access the web interface.

### Volumes

*Ensure correct read/write permissions to the config and output folders.*

```
chown -R 1000:1000 ./config ./output
```

| Path | Description |
|---|---|
| `/app/config` | Persisted configuration file (`config.toml`) |
| `/app/output` | Processed images and compiled timelapse video, one folder per run (`<name>_<YYYY-MM-DD_HHMM>`) |
| `/app/music` | *(optional, read-only is fine)* Music files (mp3, m4a, aac, wav, flac, ogg, opus) to mix into the video |

### Music

Mount a folder with audio files at `/app/music` (or set `MUSIC_DIR` to another path). In **Settings → Video** you can:

- select one or more tracks; they play one after another in the numbered order (click order),
- set the fade-out length at the end of the video,
- enable **Fit video to music**: framerate = number of images ÷ total music length, limited to a configurable
  minimum and maximum framerate. If the limit kicks in, the video ends before the music (minimum) or runs
  longer than the music (maximum); the music is always cut and faded out at whichever ends first.

### Photo sources (guided view)

In step 2 of the guided view you can limit the photos of the person to sources, as in Immich Mosaik:

- **Albums**: photos in one of the chosen albums,
- **People**: photos that also show one of the chosen people,
- **Shared with** *(optional)*: photos in an album shared with one of the chosen people in
  Immich Smart Albums.

A photo counts if it matches at least one chosen source; nothing chosen means all photos.

The **Shared with** tab is only shown when both environment variables are set:

| Variable | Description |
|---|---|
| `SMARTALBUMS_URL` | Base URL of Smart Albums as reachable from this container, e.g. `http://smartalbums:8080` |
| `CONNECT_KEY` | The connect key of Smart Albums (sent as `X-Connect-Key`), the same value Mosaik uses |

If Smart Albums can't be reached, the tab shows the error instead of the list.

### Fork image (GHCR)

This fork publishes a multi-arch image to `ghcr.io/rheckly/immich-automated-selfie-timelapse` on every push to
`happy-pc` (tags: `latest`, branch name, short SHA). Only the last 3 versions are kept.

### Settings

The default settings are quite permissive because every human being is unique.  
Please adjust image brightness filtering, eye aspect ratio etc. for the person you are processing.

<p align="center">
  <img src="resources/settings_details.png" alt="Settings">
</p>


## Additional info

- **To get the best results, I advise going into the galery view and quickly going through the images to remove bad ones, then hit "Compile video" at the bottom to recreate the video. There are always outliers that pass the quality filters. Removing bad photos makes a big difference in the final output.**
- The photo filtering is not 100% accurate and will continue to improve.
- Heartfelt thanks to the Immich team and contributors for making this project possible.
- About contribution : When I first created this project, I marked it as open to contributions. I now realize that I don't have as much time as I thought to dedicate to this project. I feel comfortable with issues being opened as it allows me to go through them at my own pace. For pull requests, I cannot guarantee a reasonable time frame for review. 
- Thank you thomaslrg for the discussions around the project.
- Thank you for the ~~200~~ ~~300~~ 600 GitHub stars !


## License

This project is open source and available under the [MIT License](LICENSE).

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=ArnaudCrl/immich-automated-selfie-timelapse&type=date&legend=top-left)](https://www.star-history.com/#ArnaudCrl/immich-automated-selfie-timelapse&type=date&legend=top-left)
