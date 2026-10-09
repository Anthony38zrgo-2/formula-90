# Lewis Hamilton inspired driver appearance

Method: built-in image generation edited the original texture atlases using the supplied helmet and suit photographs. The following describes the generation instructions; it is not an exact transcript of the prompts.

Helmet: preserve the original helmet atlas islands, hardware and layout. Replace the red shell with saturated yellow, thin red contour motifs and stars, recognizable 44 markings, Ferrari shield, UniCredit, Bitdefender and ZYN accents, and a white HP / Richard Mille visor band. Keep sufficient dark padding outside the islands. The generated image has vertical margins handled by texture coordinate alignment.

Suit: preserve every original square atlas island and its position. Create a red racing suit with white side panels, Shell and UniCredit chest patches, prominent HP chest mark, smaller Ferrari and CEVA accents, Puma shoulder and boot motifs, and LEWIS 44 with a British flag near the waist. Preserve cloth shading and seams.

Gloves: preserve the four original glove islands and their positions. Use Ferrari red backs and black palms, white Puma motifs, yellow shield accents and white 44 markings. Preserve fingers and seams.

The supplied geometry and all rig transforms are unchanged. The visor uses editable warm gold and violet vertex colors with a reflective material; its color attribute is exported as the primary vertex color set for Godot compatibility.
