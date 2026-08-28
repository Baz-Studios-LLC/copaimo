## Copaimo: The Wardens Guild

Still a world to walk and nothing yet to do in it — no Copaimo, no ranch, no guild
exams. This release is about **the person you walk it as**. The warden was a box
under a round hat; he is now a modelled character with a face, a jacket, hands, and
a walk and a run authored for him. Everything below is either getting him into the
game intact or fixing something that was visibly wrong once he was.

### There is a warden now

A delivered character model — mesh, skeleton, and a walk and a run — replaces the
placeholder. The build that brings him in does **three things and nothing else**:
renames the clips, reads his height, and settles the game around him. His bones keep
the names his artist gave them, his scale is his own, and his facing is his own.

That restraint was learned the hard way. An earlier version of this pipeline
translated the whole rig into the conventions our tools already spoke — renamed the
bones, rescaled the figure, rebuilt the bone tails — and one of those steps quietly
reinterpreted every keyframe in the run. The character came out of it dancing. A
bone's rest orientation is the frame its animation is measured in, so re-aiming a
bone redefines what every stored rotation means, without changing a single key.

Now the delivered clips are final and the tools adapt to the asset instead. It is
checked rather than claimed: the run that ships differs from the delivered file by
**at most 0.003 cm on any bone on any frame**.

### He walks and runs at the speed he was animated to

He moved like he was wading. The game was driving him at 2.90 m/s under a run
animated at 4.77, which plays the clip at **0.61× speed** — and slow motion reads as
broken long before it reads as slow. The walk had the same fault, at 0.75×.

Both speeds now come from the clips themselves, measured off their own planted feet
rather than picked by feel. Each plays at exactly 1.00×, so the stride matches the
ground he covers and the handover between walking and running follows from the two
numbers instead of being tuned against them.

### He faces the way he is going

He ran backwards. The facing had been *derived* — reasoned out from how Blender's
axes convert on export — and the reasoning had a sign error in it, which is a fine
way to be confidently wrong in a direction nobody notices until a face is attached.

The rig carries the answer itself. The model ships with a marker sitting 11 cm in
front of the face, put there to record which way the face points, and read straight
out of the published file it points the opposite way from the game's forward. Half a
turn, decided by measurement.

### He stands still properly

With no idle delivered, he needs one, and the obvious source is wrong twice over.
Holding a frame of the walk gives a stance caught mid-stride — one leg raised, hands
trailing behind. Holding the bind pose is no better: a bind is a rigging
convenience, not a pose, and this one is **asymmetric**, with the feet 7.65 cm apart
in height, the hands 26 cm apart front to back, and both forearms passing through
the waist.

So the stand is authored. Every limb is aimed at a direction built from his own
axes — his hip line, his face marker — while the feet and hands keep the orientation
they were bound at, so the soles stay flat and the hands keep their shape. The arms
then open a couple of degrees at a time until the mesh genuinely clears the body,
and he settles onto the floor by his lowest **skin** point rather than by a bone,
because a sole is what stands on a floor.

He now stands level to 0.61 cm, with his arms clear of his waist by measurement —
zero intersecting triangles, where there were 575 — and both feet on the ground.

### His colours are not speckled any more

Dark flecks in his hair, and a mismatch at his neck. Two different causes, and
neither was the mesh.

* **The speckle was missing texture padding.** His UV atlas is shattered into
  per-triangle islands, and only **53.5%** of the texture sheet is covered by them
  at all — the rest is black. Texture filtering samples across every island edge and
  picks that black up, worst where islands are smallest and most crowded, which is
  exactly the hair and the seam at the neck. Fixed by dilation, the step every bake
  pipeline ends with: each island's own colour is pushed outward into the empty
  space around it, taking coverage to 99.6%. No painted pixel is altered.
* **The orange flecks were painted.** Thirty-two of the hair's triangles have UVs
  landing on the hood-lining part of the sheet. Those, and only those, are repainted
  to the hair's own colour — deliberately narrow, because the orange trim beside the
  jacket's green is the same relationship and belongs there.

### Fixing a foot is a whole discipline

Most of this release is the feet, and most of that is measurement rather than
animation. Planted soles now lie flat on the floor and stay where they are put; the
toes bend at toe-off and not in mid-air; the legs no longer pass through each other;
and the feet no longer twist outward through the stride.

Nearly every wrong turn along the way had the same shape — **a measurement that
agreed with itself.** Travel measured against a crooked bind. Soles called flat by a
surface normal that was quietly carrying roll as well as pitch. Legs called clear by
sampling points, on frames where 246 triangles were interpenetrating. A toe bend
corrected on one axis while the axis actually at fault was the one nobody had
looked at.

The guards that catch these are stricter than they were, and four of them were put
back to their original tolerances after being widened — widening a guard to get past
your own mistake is the one thing a guard exists to prevent.

### For anyone opening the model themselves

A short walkthrough now lives in `docs/blender-by-hand.md`: how to open the
character, what the bones are called, how to pose one, and — the part that actually
matters — where a fix has to live to survive the next build.

One thing it explains is worth repeating here. Open the model and some bones appear
to shoot several metres out of his head. Those are markers, they drive no skin, and
the spikes are Blender inventing bone lengths that glTF does not store. They are
drawing only, and the viewer now hides them.
