import AppKit

let width: CGFloat = 720
let height: CGFloat = 420
let image = NSImage(size: NSSize(width: width, height: height))

image.lockFocus()

let background = NSColor(calibratedRed: 0.95, green: 0.98, blue: 0.98, alpha: 1)
background.setFill()
NSBezierPath(rect: NSRect(x: 0, y: 0, width: width, height: height)).fill()

let accent = NSColor(calibratedRed: 0.03, green: 0.45, blue: 0.48, alpha: 1)
let secondary = NSColor(calibratedWhite: 0.16, alpha: 1)
let muted = NSColor(calibratedWhite: 0.38, alpha: 1)

func drawText(_ text: String, x: CGFloat, y: CGFloat, size: CGFloat, color: NSColor, weight: NSFont.Weight = .regular) {
    let paragraph = NSMutableParagraphStyle()
    paragraph.alignment = .center
    let attributes: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: size, weight: weight),
        .foregroundColor: color,
        .paragraphStyle: paragraph
    ]
    text.draw(in: NSRect(x: x, y: y, width: width - x * 2, height: 40), withAttributes: attributes)
}

func drawArrow(from start: NSPoint, to end: NSPoint) {
    accent.setStroke()
    let path = NSBezierPath()
    path.lineWidth = 3
    path.move(to: start)
    path.line(to: end)
    path.stroke()

    let angle = atan2(end.y - start.y, end.x - start.x)
    let arrowLength: CGFloat = 12
    let arrowAngle: CGFloat = .pi / 7
    let left = NSPoint(
        x: end.x - arrowLength * cos(angle - arrowAngle),
        y: end.y - arrowLength * sin(angle - arrowAngle)
    )
    let right = NSPoint(
        x: end.x - arrowLength * cos(angle + arrowAngle),
        y: end.y - arrowLength * sin(angle + arrowAngle)
    )
    let head = NSBezierPath()
    head.lineWidth = 3
    head.move(to: left)
    head.line(to: end)
    head.line(to: right)
    head.stroke()
}

drawText("请先阅读 README-macOS.txt", x: 48, y: 318, size: 24, color: accent, weight: .semibold)
drawText("Please read README-macOS.txt first", x: 48, y: 286, size: 20, color: secondary, weight: .medium)
drawText("然后拖到 Applications", x: 48, y: 104, size: 20, color: secondary, weight: .medium)
drawText("Drag to Applications", x: 48, y: 74, size: 18, color: muted)
drawArrow(from: NSPoint(x: 284, y: 190), to: NSPoint(x: 436, y: 190))

image.unlockFocus()

guard let tiff = image.tiffRepresentation,
      let bitmap = NSBitmapImageRep(data: tiff),
      let png = bitmap.representation(using: .png, properties: [:]) else {
    fatalError("Failed to render DMG background")
}

let output = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "dmg-background.png"
try png.write(to: URL(fileURLWithPath: output))
