# Homebrew formula for git-gist
# Publish under a tap (e.g. chtnnh/homebrew-tap) as Formula/git-gist.rb
#
# After cutting a GitHub release for vX.Y.Z:
#   1. Download https://github.com/chtnnh/git-gist/archive/vX.Y.Z.tar.gz
#   2. shasum -a 256 vX.Y.Z.tar.gz
#   3. Update url + sha256 below, bump version
#   4. Commit to the tap and `brew audit --strict --online git-gist`
class GitGist < Formula
  desc "Run git commands across all child git repositories"
  homepage "https://github.com/chtnnh/git-gist"
  url "https://github.com/chtnnh/git-gist/archive/v1.1.0.tar.gz"
  sha256 "aee78b82ddbe27acae3e5d34913083fd13f978382745c02049e5974b377863cd"
  license "MIT"
  head "https://github.com/chtnnh/git-gist.git", branch: "main"

  depends_on "rust" => :build
  depends_on "git"

  def install
    system "cargo", "install", *std_cargo_args
  end

  def caveats
    <<~EOS
      Shell helpers live in the source tree under shell/.
      After install, generate completions with:
        gg completions zsh > $(brew --prefix)/share/zsh/site-functions/_gg
    EOS
  end

  test do
    assert_match "gg", shell_output("#{bin}/gg version")
  end
end
