import test from 'node:test';
import assert from 'node:assert/strict';
import { CardHidBuffer, cardSuffix } from '../src/shared/lib/cardHid.ts';
test('same HID frame is emitted for Enter and Tab, without consuming plain navigation Tab', () => {
  for (const suffix of ['Enter', 'Tab', '\r', '\n']) {
    const parser = new CardHidBuffer();
    let time = 100;
    for (const ch of '00:ab-cd0012') assert.equal(parser.feed(ch, (time += 5)), null);
    assert.equal(parser.feed(suffix, time + 5)?.code, '00:ab-cd0012');
    assert.equal(parser.feed(suffix, time + 6), null);
  }
  assert.equal(new CardHidBuffer().feed('Tab', 100), null);
});
test('capture boundary clears old data and rejects modified input', () => {
  const p = new CardHidBuffer();
  p.feed('0', 100);
  p.feed('1', 101);
  p.reset();
  assert.equal(p.feed('Enter', 102), null);
  p.feed('0', 100);
  p.feed('1', 1200);
  assert.equal(p.feed('Tab', 1201)?.code, '1');
  p.feed('2', 1202);
  p.feed('a', 1203, true);
  assert.equal(p.feed('Enter', 1204), null);
  assert.equal(cardSuffix('Escape'), null);
});

test('overlong frames are rejected as a whole, never interpreted as a shorter card', () => {
  const p = new CardHidBuffer();
  let time = 100;
  for (const c of 'X'.repeat(257) + '000123') p.feed(c, time++);
  assert.equal(p.feed('Enter', time), null);
  p.feed('0', time + 1);
  assert.equal(p.feed('Tab', time + 2)?.code, '0');
});
