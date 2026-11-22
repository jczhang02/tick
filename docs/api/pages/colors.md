# Colors

Projects, labels, and filters have a color, set with the `color` property.
Responses always return one of the color names below.

Requests can use the name or the ID. Names are lowercase and must match
exactly, so the API rejects values like `Berry_Red` or `berry red`, along with
any name or ID that isn't in the table.

We may add more colors over time. Any API client getting a color name it
doesn't know should show `charcoal`, the default color. `charcoal` will be
used as a placeholder and must not be sent back unless the user deliberately
picks it.

| ID  | Name        | Hexadecimal                                                                   |     | ID  | Name       | Hexadecimal                                                                   |
| --- | ----------- | ----------------------------------------------------------------------------- | --- | --- | ---------- | ----------------------------------------------------------------------------- |
| 30  | berry_red   | <span class="color-block" style="background-color: #B8255F"></span> `#B8255F` |     | 40  | light_blue | <span class="color-block" style="background-color: #6988A4"></span> `#6988A4` |
| 31  | red         | <span class="color-block" style="background-color: #DC4C3E"></span> `#DC4C3E` |     | 41  | blue       | <span class="color-block" style="background-color: #4180FF"></span> `#4180FF` |
| 32  | orange      | <span class="color-block" style="background-color: #C77100"></span> `#C77100` |     | 42  | grape      | <span class="color-block" style="background-color: #692EC2"></span> `#692EC2` |
| 33  | yellow      | <span class="color-block" style="background-color: #B29104"></span> `#B29104` |     | 43  | violet     | <span class="color-block" style="background-color: #CA3FEE"></span> `#CA3FEE` |
| 34  | olive_green | <span class="color-block" style="background-color: #949C31"></span> `#949C31` |     | 44  | lavender   | <span class="color-block" style="background-color: #A4698C"></span> `#A4698C` |
| 35  | lime_green  | <span class="color-block" style="background-color: #65A33A"></span> `#65A33A` |     | 45  | magenta    | <span class="color-block" style="background-color: #E05095"></span> `#E05095` |
| 36  | green       | <span class="color-block" style="background-color: #369307"></span> `#369307` |     | 46  | salmon     | <span class="color-block" style="background-color: #C9766F"></span> `#C9766F` |
| 37  | mint_green  | <span class="color-block" style="background-color: #42A393"></span> `#42A393` |     | 47  | charcoal   | <span class="color-block" style="background-color: #808080"></span> `#808080` |
| 38  | teal        | <span class="color-block" style="background-color: #148FAD"></span> `#148FAD` |     | 48  | grey       | <span class="color-block" style="background-color: #999999"></span> `#999999` |
| 39  | sky_blue    | <span class="color-block" style="background-color: #319DC0"></span> `#319DC0` |     | 49  | taupe      | <span class="color-block" style="background-color: #8F7A69"></span> `#8F7A69` |
