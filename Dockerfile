FROM gcr.io/distroless/cc-debian12
ARG TARGETPLATFORM

LABEL org.opencontainers.image.description="FoxESS home battery steering based on NextEnergy real-time prices"
LABEL org.opencontainers.image.authors="eigenein"
LABEL org.opencontainers.image.source="https://github.com/eigenein/fennec"

STOPSIGNAL SIGKILL
ENTRYPOINT ["/fennec-cli"]
ADD $TARGETPLATFORM/fennec-cli /
