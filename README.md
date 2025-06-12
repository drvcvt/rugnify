# Rugnify: Screening Helper/Annotation Tool

Rugnify is a fast and straightforward tool to take screenshots and draw on them directly. Perfect for quickly marking, highlighting, or explaining something on an image without opening a cumbersome image editing program.

![Rugnify Demo](rugnify_demo.gif)

## Features

-   **Instant Screenshot**: Starts directly with a fresh screenshot of your main display.
-   **Drawing Mode**: Switch to a mode where you can draw freely on the image.
-   **Eraser**: Quickly correct your annotations.
-   **Intuitive Zooming and Panning**: Navigate precisely through the image with the mouse wheel and mouse button.
-   **Focus Mode**: Dims the area outside your mouse cursor to draw attention to a specific point.
-   **Lightweight and Fast**: Written in Rust for optimal performance.

## Prerequisites

Ensure you have Rust and Cargo installed on your system. You can find instructions on the [official Rust website](https://www.rust-lang.org/tools/install).

Additionally, you may need some development libraries depending on your operating system.

**For Debian/Ubuntu-based systems:**
```bash
sudo apt-get install libx11-dev libxrandr-dev libxi-dev libxcursor-dev libxinerama-dev libgl1-mesa-dev
```

**For Fedora/CentOS:**
```bash
sudo dnf install libX11-devel libXrandr-devel libXi-devel libXcursor-devel libXinerama-devel mesa-libGL-devel
```

## Installation & Usage

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/drvcvt/rugnify.git
    cd rugnify
    ```

2.  **Build the project:**
    A release build is recommended for the best performance.
    ```bash
    cargo build --release
    ```

3.  **Run the application:**
    ```bash
    cargo run --release
    ```
    The application will start immediately in fullscreen mode with a current screenshot.

## Controls

The controls are simple and designed for speed:

| Action | Key / Mouse | Description |
| :--- | :--- | :--- |
| **Toggle Drawing Mode** | `Left Ctrl` Key | Toggles between drawing and navigation mode. |
| **Paint** | `Left Mouse Button` (hold) | Draws a red line (only in drawing mode). |
| **Erase** | `Right Mouse Button` (hold) | Removes drawn lines (only in drawing mode). |
| **Pan Image** | `Left Mouse Button` (hold) | Moves the visible image area (only in navigation mode). |
| **Zoom** | `Mouse Wheel` | Zooms in or out. |
| **Focus Mode** | `Left Alt` Key (hold) | Dims the area around the mouse cursor. |
| **Exit** | `Escape` Key | Closes the application. |

---

Developed with a passion for efficiency. 