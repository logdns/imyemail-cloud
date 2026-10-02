# Install imyemail-cloud on Linux

imyemail-cloud is distributed as a Debian package for 64-bit Intel/AMD (`amd64`) and ARM (`arm64`) systems.

## Install

Open the downloaded `.deb` with your system's Software app, or run:

```bash
sudo apt install ./imyemail-cloud-linux-x86_64-20260919.deb
```

Use the `arm64` filename on an ARM computer. `apt` installs the required GTK, libadwaita, libsecret, and WebKitGTK libraries, registers imyemail-cloud in the application menu, and installs the correct icon.

Launch imyemail-cloud from the application menu or run `imyemail-cloud`.

## Verify the download

Import the public key once, then verify the detached signature:

```bash
gpg --import imyemail-cloud-release.asc
gpg --verify imyemail-cloud-linux-x86_64-20260919.deb.asc imyemail-cloud-linux-x86_64-20260919.deb
sha256sum -c imyemail-cloud-linux-x86_64-20260919.deb.sha256
```

The expected primary key fingerprint is:

```text
713C 3AC1 D1EB D447 8CCF  4C70 56CD D5AB 9315 5F22
```

## Uninstall

```bash
sudo apt remove imyemail-cloud
```

Removing the package does not automatically delete your local mail database, preferences, or credentials. Remote images remain blocked by default and can be enabled in Settings → Security & Privacy.
