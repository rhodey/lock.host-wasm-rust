if (!globalThis.Intl) globalThis.Intl = {}

if (!globalThis.Intl.NumberFormat) {
  globalThis.Intl.NumberFormat = class NumberFormat {
    constructor(_locales, _options) {}
    format(value) {
      return String(value)
    }
  }
}

if (!globalThis.Intl.DateTimeFormat) {
  globalThis.Intl.DateTimeFormat = class DateTimeFormat {
    constructor(_locales, _options) {}
    format(value) {
      return value instanceof Date ? value.toISOString() : String(value)
    }
  }
}

if (!globalThis.Intl.Collator) {
  globalThis.Intl.Collator = class Collator {
    constructor(_locales, _options) {}
    compare(a, b) {
      a = String(a)
      b = String(b)
      if (a === b) return 0
      return a < b ? -1 : 1
    }
  }
}
