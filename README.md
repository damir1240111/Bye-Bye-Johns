# Bye-Bye Johns

**Bye-Bye Johns** is an independent, open-source modding utility for Hearts of Iron IV (HOI4). It is designed to act as a powerful visual constructor that simplifies and enhances the process of creating HOI4 content (such as focus trees, scripted GUIs, country events, decisions, and more).

This project was developed by an independent developer from Russia as a response and alternative to the foreign tool [HOI4 Content Maker](https://github.com/MillenniumDawn/focus-tree-creation-tool), with the goal of providing a robust, native, and user-friendly experience for creators worldwide.

### 🕊️ Disclaimer & Mission Statement
This project is created solely for entertainment, educational, and creative purposes. **Bye-Bye Johns** is strictly non-political, does not promote or propagate any political ideologies, and is entirely free of discrimination or hostility toward any nation, community, or individual. We believe in open, respectful collaboration and aim to support the global HOI4 modding community by offering a free, accessible, and high-quality utility.

---

## 🛠️ Technology Stack
The application is built using a modern, fast, and lightweight desktop stack:
* **Backend:** [Rust](https://www.rust-lang.org/) with [Tauri v2](https://tauri.app/) (delivering high performance, safety, and a minimal footprint)
* **Frontend:** [React](https://react.dev/) + [TypeScript](https://www.typescriptlang.org/) (for interactive, stateful, and modular UI components)
* **Bundler:** [Vite](https://vite.dev/) (for fast hot-module reloading and optimized builds)

---

## 🚀 Getting Started

To run and build this project locally, make sure you have the required development environments installed on your machine.

### Prerequisites

1. **Node.js** (v20 or higher recommended)
   * Download and install from [nodejs.org](https://nodejs.org/).
2. **Rust & Cargo**
   * Install the Rust toolchain via [rustup](https://rustup.rs/).
3. **C++ Build Tools (Windows)**
   * Tauri requires the C++ Build Tools for Visual Studio. You can install them by downloading the Visual Studio Installer and selecting the **"C++ build tools"** workload (including the Windows SDK).

### Installation & Development Run

1. **Clone the repository:**
   ```bash
   git clone git@github.com:damir1240111/Bye-Bye-Johns.git
   cd Bye-Bye-Johns
   ```

2. **Install frontend dependencies:**
   ```bash
   npm install
   ```

3. **Start the application in development mode:**
   ```bash
   npm run tauri dev
   ```
   This will start the frontend dev server and compile/launch the Rust desktop window.

### Building for Production

To build a standalone executable/installer (e.g., `.exe` or `.msi` on Windows):
```bash
npm run tauri build
```
The compiled assets will be placed in `src-tauri/target/release/bundle/`.

---

## 🤝 Contributing
Contributions, feature requests, and feedback are welcome! Feel free to open an issue or submit a pull request. Let's make HOI4 modding more accessible together!
