cask "macos-openbooklet" do
  version "1.0.0"
  sha256 "0577d65cb0892ce16b3f5d25f11e0c495bed8c37c7276252144ffa1efac2fbb8"

  url "https://github.com/SilverFeit/MacOS-OpenBooklet/releases/download/v#{version}/MacOS-OpenBooklet-#{version}.dmg"
  name "MacOS OpenBooklet"
  desc "Open-source native Rust & Quartz 2D vector PDF booklet imposition, bookbinding signatures, cut & stack, and duplex calibration studio for macOS"
  homepage "https://github.com/SilverFeit/MacOS-OpenBooklet"

  depends_on macos: :sonoma
  depends_on arch: :arm64

  app "OpenBooklet.app"
  binary "#{appdir}/OpenBooklet.app/Contents/MacOS/openbooklet"

  if respond_to?(:postflight_steps)
    postflight_steps do
      run "/usr/bin/xattr",
          args: ["-cr", "{{appdir}}/OpenBooklet.app"],
          writable_paths: ["OpenBooklet.app"],
          writable_base: :appdir
    end
  else
    postflight do
      system_command "/usr/bin/xattr",
                     args: ["-cr", "#{appdir}/OpenBooklet.app"],
                     sudo: false
    end
  end

  zap trash: [
    "~/Library/Preferences/com.silverfeit.openbooklet.plist",
  ]
end
