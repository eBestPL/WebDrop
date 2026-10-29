// Synthetic fixtures only. Regenerate with swiftc -framework ImageIO -framework CoreGraphics.
import Foundation
import CoreGraphics
import ImageIO

let output = CommandLine.arguments[1]
try FileManager.default.createDirectory(atPath: output, withIntermediateDirectories: true)
let width = 80, height = 60
var bytes = [UInt8]()
for y in 0..<height {
    for _ in 0..<width { bytes += y < height / 2 ? [240, 20, 30, 255] : [20, 30, 240, 255] }
}
let provider = CGDataProvider(data: Data(bytes) as CFData)!
let image = CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32,
    bytesPerRow: width * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
    bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue), provider: provider,
    decode: nil, shouldInterpolate: false, intent: .defaultIntent)!
for (name, type, count, orientation) in [
    ("colors.tiff", "public.tiff", 1, 1),
    ("pages.tiff", "public.tiff", 2, 1),
    ("colors.heic", "public.heic", 1, 1),
    ("rotated.heic", "public.heic", 1, 6)
] {
    let url = URL(fileURLWithPath: output).appendingPathComponent(name)
    guard let dest = CGImageDestinationCreateWithURL(url as CFURL, type as CFString, count, nil) else { fatalError("Cannot create \(type)") }
    for _ in 0..<count {
        CGImageDestinationAddImage(dest, image, [kCGImagePropertyOrientation: orientation,
            kCGImageDestinationLossyCompressionQuality: 0.95] as CFDictionary)
    }
    guard CGImageDestinationFinalize(dest) else { fatalError("Cannot write \(name)") }
}
