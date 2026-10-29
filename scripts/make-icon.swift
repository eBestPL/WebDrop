import AppKit

// Draw at every native icon size so the silhouette stays crisp in Finder and the Dock.
func color(_ red: CGFloat, _ green: CGFloat, _ blue: CGFloat, _ alpha: CGFloat = 1) -> NSColor {
    NSColor(srgbRed: red, green: green, blue: blue, alpha: alpha)
}

func drawIcon(_ context: CGContext) {
    let tile = NSBezierPath(roundedRect: NSRect(x: 64, y: 64, width: 896, height: 896), xRadius: 202, yRadius: 202)
    NSGraphicsContext.saveGraphicsState()
    let shadow = NSShadow()
    shadow.shadowColor = color(0.02, 0.08, 0.22, 0.30)
    shadow.shadowOffset = NSSize(width: 0, height: -14)
    shadow.shadowBlurRadius = 22
    shadow.set()
    color(0.03, 0.22, 0.65).setFill()
    tile.fill()
    NSGraphicsContext.restoreGraphicsState()

    NSGradient(colors: [color(0.025, 0.12, 0.38), color(0.04, 0.37, 0.91), color(0.10, 0.68, 0.94)])!.draw(in: tile, angle: 68)
    NSGraphicsContext.saveGraphicsState()
    tile.addClip()
    let glow = NSBezierPath(ovalIn: NSRect(x: 380, y: 580, width: 790, height: 600))
    NSGradient(starting: color(0.40, 1, 0.91, 0.30), ending: color(0.40, 1, 0.91, 0))!.draw(in: glow, relativeCenterPosition: NSPoint(x: 0, y: 0))
    NSGraphicsContext.restoreGraphicsState()
    color(1, 1, 1, 0.22).setStroke()
    tile.lineWidth = 2
    tile.stroke()

    // The drop is also a picture: one simple mark instead of separate badges.
    let drop = NSBezierPath()
    drop.move(to: NSPoint(x: 512, y: 828))
    drop.curve(to: NSPoint(x: 247, y: 460), controlPoint1: NSPoint(x: 450, y: 740), controlPoint2: NSPoint(x: 247, y: 591))
    drop.curve(to: NSPoint(x: 512, y: 199), controlPoint1: NSPoint(x: 247, y: 306), controlPoint2: NSPoint(x: 365, y: 199))
    drop.curve(to: NSPoint(x: 777, y: 460), controlPoint1: NSPoint(x: 659, y: 199), controlPoint2: NSPoint(x: 777, y: 306))
    drop.curve(to: NSPoint(x: 512, y: 828), controlPoint1: NSPoint(x: 777, y: 591), controlPoint2: NSPoint(x: 574, y: 740))
    drop.close()
    NSGraphicsContext.saveGraphicsState()
    let dropShadow = NSShadow()
    dropShadow.shadowColor = color(0.015, 0.10, 0.30, 0.40)
    dropShadow.shadowOffset = NSSize(width: 0, height: -18)
    dropShadow.shadowBlurRadius = 28
    dropShadow.set()
    color(0.90, 1, 1).setFill()
    drop.fill()
    NSGraphicsContext.restoreGraphicsState()
    NSGradient(colors: [color(0.48, 0.90, 0.98), color(0.89, 1, 1), .white])!.draw(in: drop, angle: 90)

    NSGraphicsContext.saveGraphicsState()
    drop.addClip()
    let distant = NSBezierPath()
    distant.move(to: NSPoint(x: 245, y: 340))
    distant.line(to: NSPoint(x: 435, y: 543))
    distant.curve(to: NSPoint(x: 461, y: 543), controlPoint1: NSPoint(x: 443, y: 552), controlPoint2: NSPoint(x: 453, y: 552))
    distant.line(to: NSPoint(x: 738, y: 263))
    distant.line(to: NSPoint(x: 245, y: 170))
    distant.close()
    NSGradient(starting: color(0.10, 0.62, 0.86), ending: color(0.13, 0.78, 0.82))!.draw(in: distant, angle: 90)

    let mountain = NSBezierPath()
    mountain.move(to: NSPoint(x: 360, y: 180))
    mountain.line(to: NSPoint(x: 601, y: 463))
    mountain.curve(to: NSPoint(x: 629, y: 463), controlPoint1: NSPoint(x: 610, y: 473), controlPoint2: NSPoint(x: 620, y: 473))
    mountain.line(to: NSPoint(x: 800, y: 277))
    mountain.line(to: NSPoint(x: 800, y: 170))
    mountain.close()
    NSGradient(starting: color(0.02, 0.25, 0.69), ending: color(0.05, 0.49, 0.88))!.draw(in: mountain, angle: 90)
    color(0.08, 0.43, 0.81).setFill()
    NSBezierPath(ovalIn: NSRect(x: 551, y: 548, width: 75, height: 75)).fill()
    NSGraphicsContext.restoreGraphicsState()

    color(1, 1, 1, 0.70).setStroke()
    drop.lineWidth = 3
    drop.stroke()
}

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
        drawIcon(context)
        NSGraphicsContext.restoreGraphicsState()
        let suffix = scale == 2 ? "@2x" : ""
        let path = "\(output)/icon_\(size)x\(size)\(suffix).png"
        try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: path))
    }
}
