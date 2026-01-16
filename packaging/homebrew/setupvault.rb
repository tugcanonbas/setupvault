class Setupvault < Formula
  desc "Local-first system documentation with rationale capture"
  homepage "https://tugcanonbas.github.io/setupvault/"
  url "https://github.com/tugcanonbas/setupvault/archive/refs/tags/0.1.0.tar.gz"
  sha256 "d1a2f4d502466ad911e02ae1de9131b19dcc4d073f2adf65be58e04529059783"
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
