# Stage 1: Build frontend
FROM node:22-alpine AS frontend-build
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build Rust binary
FROM ubuntu:24.04 AS rust-build
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates \
    cmake g++ make pkg-config \
    libssl-dev libdlib-dev libatlas-base-dev liblapack-dev \
    && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Cache dependency build
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && \
    echo 'fn main() { println!("dummy"); }' > src/main.rs && \
    echo '' > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# Build info shown in the UI and at /api/version (declared after the
# dependency cache so a new version doesn't rebuild all crates)
ARG APP_VERSION=dev
ARG GIT_SHA=
ARG BUILD_DATE=
ENV APP_VERSION=${APP_VERSION} GIT_SHA=${GIT_SHA} BUILD_DATE=${BUILD_DATE}

# Build real binary
COPY src/ src/
RUN touch src/main.rs src/lib.rs && cargo build --release

# Only the build artifacts, for scripts/deploy-nas.sh
# (docker build --target dev-export --output type=local,dest=build/nas .)
FROM scratch AS dev-export
COPY --from=rust-build /app/target/release/immich-timelapse /
COPY --from=frontend-build /app/frontend/dist/ /dist/

# Stage 3: Runtime image
# (Montserrat comes as OTF and TTF: only the TTF is kept, without Alternates)
FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    libdlib19.1 \
    libatlas3-base \
    liblapack3 \
    libssl3 \
    libgomp1 \
    ca-certificates \
    curl \
    bzip2 \
    fonts-dejavu-core \
    fonts-liberation2 \
    fonts-open-sans \
    fonts-lato \
    fonts-roboto-unhinted \
    fonts-montserrat \
    fonts-bebas-neue \
    && rm -rf /var/lib/apt/lists/* \
    && rm -rf /usr/share/fonts/opentype/montserrat \
    && rm -f /usr/share/fonts/truetype/montserrat/MontserratAlternates-*

# Fonts not packaged in Ubuntu, from google/fonts at a fixed commit (OFL / Apache 2.0)
RUN mkdir -p /usr/share/fonts/truetype/google && cd /usr/share/fonts/truetype/google && \
    base=https://raw.githubusercontent.com/google/fonts/29b4ec6df2522a8aee279414b78a07910b065874/ofl && \
    curl -fsSLO $base/anton/Anton-Regular.ttf && \
    curl -fsSLO $base/archivoblack/ArchivoBlack-Regular.ttf && \
    curl -fsSLO $base/lilitaone/LilitaOne-Regular.ttf && \
    curl -fsSL -o Fredoka-Regular.ttf "$base/fredoka/Fredoka%5Bwdth,wght%5D.ttf" && \
    apache=https://raw.githubusercontent.com/google/fonts/29b4ec6df2522a8aee279414b78a07910b065874/apache && \
    curl -fsSLO $apache/chewy/Chewy-Regular.ttf && \
    curl -fsSLO $apache/luckiestguy/LuckiestGuy-Regular.ttf && \
    printf '%s\n' \
      "a4ba3a92350ebb031da0cb47630ac49eb265082ca1bc0450442f4a83ab947cab  Anton-Regular.ttf" \
      "dd9a89a019b4849f66ab75455fe7bdf931311042cbb0f0f97acc061539703180  ArchivoBlack-Regular.ttf" \
      "f5b641c45c69d772ee4eda687bc9fda411d5cad6b0b45371491da4580cbc8d59  LilitaOne-Regular.ttf" \
      "2ba02e68b152868aef9ba28e24b3648c7d457fe6f25c761f2c2c53fb61a73fc8  Fredoka-Regular.ttf" \
      "7cf75ea288f82fd20badea8ab4da7a656a96a7277c170811e813b3d3d6294147  Chewy-Regular.ttf" \
      "cfbdd68a039f92df51cf3721506af6242e64594c6325fe0bedbeff3fe385d980  LuckiestGuy-Regular.ttf" \
    | sha256sum -c

WORKDIR /app

# Download models
RUN mkdir -p models && \
    curl -fsSL -o models/dmhead_nomask_Nx3x224x224.onnx \
        https://github.com/PINTO0309/DMHead/releases/download/1.1.2/dmhead_nomask_Nx3x224x224.onnx && \
    echo "8dd5643923680b3a8e27507c4fc3d7331e0044a2554169e5658770d5b27fd122  models/dmhead_nomask_Nx3x224x224.onnx" | sha256sum -c && \
    curl -fsSL -o /tmp/shape_predictor.dat.bz2 \
        https://dlib.net/files/shape_predictor_68_face_landmarks.dat.bz2 && \
    echo "7d6637b8f34ddb0c1363e09a4628acb34314019ec3566fd66b80c04dda6980f5  /tmp/shape_predictor.dat.bz2" | sha256sum -c && \
    bzip2 -d < /tmp/shape_predictor.dat.bz2 > models/shape_predictor_68_face_landmarks.dat && \
    rm /tmp/shape_predictor.dat.bz2 && \
    apt-get purge -y --autoremove bzip2

# Copy artifacts from build stages
COPY --from=rust-build /app/target/release/immich-timelapse ./
COPY --from=frontend-build /app/frontend/dist/ ./frontend/dist/

RUN mkdir -p output config music

# 5000: app (internal only), 5001: shared links only (may be published)
EXPOSE 5000 5001
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fs http://localhost:5000/api/health || exit 1
CMD ["./immich-timelapse"]
