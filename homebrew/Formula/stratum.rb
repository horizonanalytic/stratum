# Homebrew formula for Stratum programming language
# https://github.com/horizon-analytic/stratum
#
# Installation:
#   brew tap horizon-analytic/stratum
#   brew install stratum
#
# Installation with options:
#   brew install stratum                  # Data tier (core + data, default)
#   brew install stratum --with-gui       # GUI tier (+ GUI framework)
#   brew install stratum --with-full      # Full tier (+ Workshop IDE, LSP)
#
# For development builds:
#   brew install --HEAD stratum

class Stratum < Formula
  desc "Goldilocks programming language with native data ops and GUI"
  homepage "https://stratum-lang.dev"
  url "https://github.com/horizon-analytic/stratum/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"
  head "https://github.com/horizon-analytic/stratum.git", branch: "main"

  livecheck do
    url :stable
    strategy :github_latest
  end

  # Tiered installation options (for custom tap - not supported in homebrew-core)
  # Default: Data tier (core + data operations) ~45 MB
  option "with-gui", "Include GUI framework support (~80 MB)"
  option "with-full", "Include Workshop IDE and LSP (~120 MB)"

  depends_on "rust" => :build
  depends_on "pkg-config" => :build

  # macOS system libraries
  uses_from_macos "curl"
  uses_from_macos "zlib"

  def install
    # Determine which Cargo features to enable based on options
    cargo_features = []

    if build.with?("full")
      # Full tier: GUI + Workshop + LSP
      cargo_features = ["gui", "workshop", "lsp"]
    elsif build.with?("gui")
      # GUI tier: just GUI framework
      cargo_features = ["gui"]
    end
    # Default: no optional features (Data tier - core + data always included)

    # Build arguments
    args = std_cargo_args(path: "crates/stratum-cli")

    if cargo_features.any?
      # Build with selected features, disabling defaults to avoid full install
      system "cargo", "install",
             "--no-default-features",
             "--features", cargo_features.join(","),
             *args
    else
      # Data tier: build without optional features
      system "cargo", "install",
             "--no-default-features",
             *args
    end

    # Generate and install shell completions
    generate_completions_from_executable(bin/"stratum", "completions")
  end

  def caveats
    tier = if build.with?("full")
             "Full (CLI, Data, GUI, Workshop IDE, LSP)"
           elsif build.with?("gui")
             "GUI (CLI, Data, GUI framework)"
           else
             "Data (CLI, Data operations)"
           end

    <<~EOS
      Stratum has been installed with the #{tier} tier.

      To reinstall with different features:
        brew reinstall stratum --with-full    # Full installation
        brew reinstall stratum --with-gui     # GUI tier
        brew reinstall stratum                # Data tier (default)

      Shell completions have been installed.

      For bash, add to your ~/.bashrc:
        source $(brew --prefix)/etc/bash_completion.d/stratum

      For zsh, completions are automatically loaded.

      For fish, completions are automatically loaded.

      To get started with Stratum:
        stratum repl     # Start the interactive REPL
        stratum init     # Create a new project
        stratum --help   # Show all commands
    EOS
  end

  test do
    # Verify the binary runs and shows version
    assert_match "stratum #{version}", shell_output("#{bin}/stratum --version")

    # Verify REPL banner works (non-interactive)
    assert_match "Stratum", shell_output("echo ':q' | #{bin}/stratum repl 2>&1", 0)

    # Create and run a simple Stratum program
    (testpath/"hello.strat").write <<~STRAT
      fx main() {
        print("Hello from Stratum!")
      }
    STRAT
    assert_match "Hello from Stratum!", shell_output("#{bin}/stratum run #{testpath}/hello.strat")

    # Verify shell completions are generated
    assert_match "stratum", shell_output("#{bin}/stratum completions bash")
  end
end
