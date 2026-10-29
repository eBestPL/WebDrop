import AppKit

let output = CommandLine.arguments[1]
try FileManager.default.createDirectory(atPath: output, withIntermediateDirectories: true)
for size in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let px = size * scale
        let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: px, pixelsHigh: px, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: px * 4, bitsPerPixel: 32)!
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
        let context = NSGraphicsContext.current!.cgContext
        context.scaleBy(x: CGFloat(px) / 1024, y: CGFloat(px) / 1024)
        let bounds = CGRect(x: 64, y: 64, width: 896, height: 896)
        let shape = NSBezierPath(roundedRect: bounds, xRadius: 205, yRadius: 205)
        NSColor(calibratedRed: 0.39, green: 0.31, blue: 0.87, alpha: 1).setFill()
        shape.fill()
        NSGradient(starting: NSColor(calibratedRed: 0.57, green: 0.47, blue: 0.97, alpha: 1), ending: NSColor(calibratedRed: 0.32, green: 0.24, blue: 0.75, alpha: 1))!.draw(in: shape, angle: -65)
        NSColor.white.withAlphaComponent(0.96).setStroke()
        let frame = NSBezierPath(roundedRect: NSRect(x: 245, y: 277, width: 534, height: 470), xRadius: 54, yRadius: 54)
        frame.lineWidth = 35
        frame.stroke()
        NSColor.white.setFill()
        NSBezierPath(ovalIn: NSRect(x: 322, y: 583, width: 81, height: 81)).fill()
        let hills = NSBezierPath()
        hills.move(to: NSPoint(x: 282, y: 333))
        hills.line(to: NSPoint(x: 455, y: 530))
        hills.line(to: NSPoint(x: 568, y: 418))
        hills.line(to: NSPoint(x: 641, y: 496))
        hills.line(to: NSPoint(x: 744, y: 356))
        hills.lineWidth = 34
        hills.lineJoinStyle = .round
        hills.lineCapStyle = .round
        hills.stroke()
        NSColor(calibratedRed: 0.60, green: 0.98, blue: 0.78, alpha: 1).setFill()
        NSBezierPath(ovalIn: NSRect(x: 661, y: 180, width: 196, height: 196)).fill()
        NSColor(calibratedRed: 0.13, green: 0.36, blue: 0.28, alpha: 1).setStroke()
        let arrow = NSBezierPath()
        arrow.move(to: NSPoint(x: 759, y: 326))
        arrow.line(to: NSPoint(x: 759, y: 234))
        arrow.move(to: NSPoint(x: 720, y: 274))
        arrow.line(to: NSPoint(x: 759, y: 234))
        arrow.line(to: NSPoint(x: 798, y: 274))
        arrow.lineWidth = 18
        arrow.lineCapStyle = .round
        arrow.lineJoinStyle = .round
        arrow.stroke()
        NSGraphicsContext.restoreGraphicsState()
        let suffix = scale == 2 ? "@2x" : ""
        let path = "\(output)/icon_\(size)x\(size)\(suffix).png"
        try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: path))
    }
}
