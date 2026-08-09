# Starroom UI Motion + Performance Specification

## Principle
UI animation must never delay image feedback.

## Motion classes
- Micro: 100–140 ms
- Normal: 160–220 ms
- Macro: 220–280 ms
- Easing: cubic-bezier(0.2, 0, 0, 1)

## Never animate
- slider value itself
- histogram data with long interpolation
- mask brush stroke behind pointer
- photo pan/zoom in a way that adds pointer latency

## Prefer compositor properties
- opacity
- transform

Avoid layout-thrashing animation on width/height/left/top when transform can represent the transition.

## Glass cost rules
Persistent panels use faux-glass surfaces. Real `backdrop-filter` is transient only.

## Reduced motion
When `prefers-reduced-motion: reduce`: no translate entrance, opacity <=100 ms, no icon morph requiring path animation, no decorative gradient movement.

## UI rendering budget
During continuous image slider drag priority is: pointer/slider event -> image preview -> mask visualization -> UI animation -> histogram -> decorative effects. Decorative tasks yield first.

## Quality mode independence
UI quality, preview render quality, and AI quality are three separate settings internally. Auto may coordinate them, but never conflate them into one hidden knob.
