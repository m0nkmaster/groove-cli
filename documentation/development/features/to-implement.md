# Features Ideas yet to create

1.  Handle long samples (loops)
2.  Clipping vs Oneshot
3.  Chopping and Cropping (ms?)
4.  Edit patterns
5.  Color
6.  Simpler naming - double quotes not required, file extension not required, assumes correct folders
7.  Chaining syntax - track(1).color(blue).sample(kick).delay(3).solo(1).play (baseline chaining parser implemented; add higher-level helpers like color)
8.  Variations to tracks
9.  Better UI/UX for error messages
10. Advanced chaining helpers (conditional logic, macros once baseline chaining is stable)
11. Autocomplete for sample selection
12. Balance controls
13. Lots of 'effects/modifiers' per track (delay, reverb, flange...)
14. Envelopes
15. LFOs
16. Gates
17. Whole song effects
18. AI - Of course! Use to create patterns
19. Performance commands (quick variation switch, hi-pass, stutter)
20. Change at end of 'bar'
21. View tracks playing (expand list?) Only list specific tracks `list 1 2 3` for example or `list(1,2,3)` in chaining syntax
22. Basic logic (only in chaining) - e.g. if(1, playhead = 1).pattern(1, "x...").then.pattern(1, "xxxx").wait(4,stop)
23. Sensible defaults - missing params in YAML cause no issues