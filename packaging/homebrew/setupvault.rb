class Setupvault < Formula
  desc "Local-first system documentation with rationale capture"
  homepage "https://tugcanonbas.github.io/setupvault/"
  url "https://github.com/tugcanonbas/setupvault/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "REPLACE_WITH_TARBALL_SHA256"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    vault = testpath/"vault"
    system "#{bin}/setupvault", "init", "--path", vault
    assert_predicate vault/"entries", :exist?
    assert_predicate vault/".state", :exist?
  end
end
