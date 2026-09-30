/** Browser demos use a frozen sample for region selection and annotation. */
export function previewCaptureImage(): Promise<HTMLImageElement> {
  const width = window.innerWidth, height = window.innerHeight;
  const escape = (value: string) => value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
  const lines = ['function makeYourPoint() {', '  const capture = screen.selectRegion();', '  capture.annotate("This is the part I mean");', '  clipboard.copy(capture);', '}', '', '// Capture. Explain. Paste into your conversation.'];
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width * 2}" height="${height * 2}" viewBox="0 0 ${width} ${height}"><rect width="100%" height="100%" fill="#edf2ef"/><rect x="40" y="80" width="${Math.max(100, width - 80)}" height="${Math.max(100, height - 180)}" rx="16" fill="#202b30"/><text x="70" y="124" fill="#a3e9d1" font-family="sans-serif" font-size="16">Glassboard · Screenshot preview</text>${lines.map((line, i) => `<text x="70" y="${180 + i * 32}" fill="${i === 6 ? '#8ea69b' : '#e3ede7'}" font-family="monospace" font-size="16">${escape(line)}</text>`).join('')}</svg>`;
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error('Could not load the screenshot preview.'));
    image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  });
}

export async function copyPreviewCapture(image: Promise<Blob>): Promise<void> {
  if (!navigator.clipboard?.write || typeof ClipboardItem === 'undefined') {
    throw new Error('Image copy is unavailable in this browser. Use the desktop app to copy captures.');
  }
  // Supply the promise during the user gesture, including on Safari.
  await navigator.clipboard.write([new ClipboardItem({ 'image/png': image })]);
}
