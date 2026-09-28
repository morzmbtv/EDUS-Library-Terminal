/** HID framing only. Card normalization and HMAC belong exclusively to Rust. */
export function cardSuffix(key: string): 'Enter' | 'Tab' | null {
  return key === 'Enter' || key === '\r' || key === '\n'
    ? 'Enter'
    : key === 'Tab' || key === '\t'
      ? 'Tab'
      : null;
}
export class CardHidBuffer {
  private value = '';
  private last = 0;
  private overflow = false;
  reset() {
    this.value = '';
    this.last = 0;
    this.overflow = false;
  }
  feed(key: string, now: number, modified = false): { code: string; suffix: 'Enter' | 'Tab' } | null {
    if (modified) {
      this.reset();
      return null;
    }
    if (now - this.last > 1000) this.reset();
    this.last = now;
    const suffix = cardSuffix(key);
    if (suffix) {
      const code = this.overflow ? '' : this.value;
      this.reset();
      return code ? { code, suffix } : null;
    }
    if (key.length === 1) {
      if (this.value.length >= 256) this.overflow = true;
      else if (!this.overflow) this.value += key;
    }
    return null;
  }
}
