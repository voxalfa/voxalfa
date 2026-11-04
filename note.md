# Voxalfa overview

Voxalfa is a declarative language for composing and storing tonic solfa partitions.

## Fundamentals

The language is based around line-wise declarations and uses **markers** to denote the type of the declaration, markers are delimited by brackets (`[<marker>]`).

There are two types of declarations:

- key-value: for assignment (`key={value}`)
- token list: custom syntax (solfa, verses)

The language has 4 main primitive types:

- number: integer or float
- string: delimited by double quotes
- boolean: true or false
- builtins: predifined constans

Builtins are:

- `Key`: `(A|B|C|D|E|F|G)(#|b)?`
- `StaticTempo`: grave | largo | adaigo | andate | moderato | allegro | vivace | presto | number
- ExtendedTempo: Rit | Accel | StaticTempo
- Voice: S | A | T | B
- Jump: DS | DC | DSC | DSF | DCC | DCF
- Mark: S | C | F | TC
- Touch: stc | frm | acc
- Dynamic: ppp | pp | p | mp | mf | f | ff | fff | cre | dec

Some builtins like dynamics and touches can also be derived by appending a time index or range (example: `p:1`, `cre:1.5..3`). Timed values are zero-based and relative to sections and solfa notes which will be explained later. Dynamics and touches written without time indices default to 0.

The language use a JSX like syntax for declaring values, though delimiters are optional in case of strings

```
key={value}
```

A list can be built by separating primitives with comma

```
list={first, second}
```

Inline comments are supported by adding `;` before some text.

There is also a special type of comment called language directives that give some context to the compiler.

```
;; @<directive> <optional_argument>
```

A voxalfa file is composed by two main parts delimited by the `---` separator.

- header: static and fully typed metadata about the partition
- body: the actual composition blocks

## Header

The header is composed by metadata and initial parameter declarations.

### Metadata

Metadata are key-values declarations denoted by the `#` marker.

Supported metadata are:

- title: `string`
- author: `string | string[]`
- composer: `string | string[]`
- verses: `number`
- meter: `number[]`
- release: `number`
- language: `string`
- description: `string`
- tags: `string[]`

### Parameters

Initial parameters are used to describe how the partition is played, they are denoted by the `$` marker. Every parameter except voices can be overriden in the body.

Supported parameters are:

- key: Key
- time: `[number, number]`
- tempo: `StaticTempo`
- voices: `Voice[]`

## Body

The body is composed by sections which are the main building blocks of the solfa DSL. They are separated by the `--` or `<<` delimiter and they can have one or multiple sub-sections separated by `++`.

Sections are a convinient way to write the solfa partition through multiple lines, they don't affect how the partition is rendered. Sub-sections are used to split declarations to build a multi-lyrics (per voice) system.

Section parameters are defined using the `$` marker at the top level (in the first sub-section), supported parameters are:

- label: string
- time: `[number, number]`
- tempo: `ExtendedTempo`
- ending: `number`
- key: `Key`
- jump: `Jump`
- mark: `Mark`
- touches: `Timed<Touch>[]`
- repeat: `number`

There are also sub-section only parameters:

- dynamics: `Timed<Dynamic>[]`

### Solfa

Solfa declarations are denoted by `[<voice>]` and ends with a `||`. Pulses are the main components of solfa lines. A represents is a beat relative to the time signature, it is composed by one note or multiple notes using beat dividers.

A pulse starts with an accent and there are 3 types of them:

- `|`: strong accent, used at the beginning of a measure
- `!`: medium accent, used at the 2nd or 3rd position depending on the time signature
- `:` weak accent, otherwise

A note is a token defined by: `(d|r|m|f|s|l|t|d)(a|i)?(+|-\d)?`

It has three parts:

- base: the base note
- variation: lowered (a), raised (i)
- octave: up (+), down (-)

There are three types of beat dividers:

- `,`: quarter (1/4)
- `.`: half (1/2)
- `.,`: 3/4

Successive notes without dividers are valid and are implicitely divided.

`|d .r` is the same as `|dr`

To define a triplet:

`|d ,r ,m` or `|d r m`

Time indices are relative to the section start and note durations. Let's consider the following examples.

#### Example 1

```
[$] dyanmics={p:2}

;    0  1  2  3
[S] |d :r !m :f
```

Each pulse is a whole note so their duration is one, so the dynamic `p:2` indexes the third pulse.

#### Example 2

```
[$] dyanmics={cre:0.5..1}

;    0 0.25 0.5 0.75 1  1.5
[S] |d  ,r  .m  ,f  :s  .l
```

Here the pulse is devided into multiple notes, and a range is used. Ranges are inclusive, so `cre:0.5..1` spans accross the third note to the fifth one.

### Example 3

Indices are reset when starting a new section with the `--` delimiter

```
;    0  1  2  3
[S] |d :r !m :f
--
;    0  1  2  2
[S] |s :l !t :d+1
```

It is not the case when using the `<<` delimiter

```
;    0  1  2  3
[S] |d :r !m :f
<<
;    4  5  6  7
[S] |s :l !t :d+1
<<
;    8    9  10   11
[S] |r+1:m+1!f+1:s+1
```

### Lyrics

Lyrics are relative to notes and are denoted by `[<verse>]`. Visual alignment is not needed with solfa lines, lyrics are attached explicitely to notes by following special rules.

By default, a lyric token match a note.

```
[S] |d .r :m .f !s .l :t .d+1
[1] do re mi fa so la ti do
```

Tokens are separated by **operators** to reflect spacing, concatenation, and linebreaks so that they can be rendered as normal text without the solfa. In the previous example, all of the tokens were separated by spaces.

There are 3 types of operators:

- ` `: space
- `_`: concat
- `\`: linebreak

Let's consider the following example.

```
[S] |d :r !m :f
[1] do re_mi\fa
```

It'll be rendered as:

```
do remi
fa
```

When multiple voices are used and their beat division don't match within a pulse, the token will match the whole pulse instead of a single note

```
[S] |d .d :m :f
[A] |d    :m :f
[1] do    mi fa
```

Parenthesis can be used to group multiple words separated by operators under a single note.

```
[S] |d :r !m :f
[1] (do o) re mi fa
```

The anchor `@@` can be used to join the last token of a line to the first token in the next section, it is preceeded by one operator.

```
[S] |d :r !m :f
[1] do re mi fa @@
--
[S] |s :l !t :d+1
[1] so la ti do
```

The span modifier `@` can be used to increase the span of a token (`<token>@<span>`).

```
[S] |d :- !r: m
[1] do@2 re mi
```

## Example 1

```voxalfa
;; @version 0.1.0-alpha

[#] title="INDRO NY ARON'NY MANDANAO"
[#] verses={1}
[#] description="Antema 5"
[#] tags={"antema","evangelic"}
[#] language="mg"

[$] key={Ab} | time={4,4} | tempo={100} | voices={S,A,T,B}

---

[S] |s-1                :s-1          .,s-1 !s-1                :s-1          .,s-1 ||
[A] |m-1                :m-1          .,m-1 !m-1                :m-1          .,m-1 ||
[T] |d                  :d            .,d   !d                  :d            .,d   ||
[B] |d-1                :d-1          .,d-1 !d-1                :d-1          .,d-1 ||

[1] I_                  ndro           ny   A_                  ron'_          ny   @@

--

[S] |l-1                :l-1                !l-1                :                   ||
[A] |f-1                :f-1                !f-1                :                   ||
[T] |d                  :d                  !d                  :                   ||
[B] |f-1                :f-1                !f-1                :                   ||

[1] ma_                 nda_                nao,                ~                   @@

--

[S] |r                  :-                  !t-1                :-                  ||
[A] |l-1                :-                  !s-1                :-                  ||
[T] |f                  :-                  !r                  :-                  ||
[B] |r-1                :-                  !s-1                :-                  ||

[1] ry@2                                    Je@2_                                   @@

--

[S] |d            .,s-1 :s-1                !                   :                   ||
[A] |s-1          .,m-1 :m-1                !                   :                   ||
[T] |m            .,d   :d                  !                   :                   ||
[B] |d-1          .,d-1 :d-1                !                   :                   ||

[1] ro_            sa_  lema!\              ~                   ~                   @@

--

[S] |m                  :m                  !m        .m        :m            .,m   ||
[A] |s-1                :s-1                !s-1          .,s-1 :s-1          .,s-1 ||
[T] |d                  :d                  !d        .d        :d            .,d   ||
[B] |d                  :d                  !d        .d        :d            .,d   ||

[1] Mi_                 a_                  dàna                ka             ma_  @@

--

[S] |f                  :-                  !-                  :                   ||
[A] |l-1                :-                  !-                  :                   ||
[T] |d                  :-                  !-                  :                   ||
[B] |f-1                :-                  !-                  :                   ||

[1] ndira@3                                                     ~                   @@

--

[S] |r                  :-                  !`d                 :r`                 ||
[A] |`s-1               :f-1`               !`m-1               :s-1`               ||
[T] |t-1                :-                  !`d                 :t-1`               ||
[B] |s-1                :-                  !`l-1               :s-1`               ||

[1] fa@2_                                   hi@2_                                   @@

--

[S] |m                  :-                  !-                  :                   ||
[A] |s-1                :-                  !-                  :                   ||
[T] |d                  :-                  !-                  :                   ||
[B] |d-1                :-                  !-                  :                   ||

[1] zay@3\                                                      ~                   @@

--

[S] |                   :                   !                   :                   ||
[A] |                   :                   !                   :                   ||
[T] |m                  :m            .,m   !m                  :         .m        ||
[B] |d                  :d            .,d   !d                  :         .d        ||

[1] Ma_                 ndry           to_  koa                 ~         ny        @@

--

[S] |                   :                   !                   :                   ||
[A] |                   :                   !                   :                   ||
[T] |f                  :m            .,m   !r                  :         .r        ||
[B] |r                  :d            .,d   !s-1                :         .s-1      ||

[1] ta_                 ny             i_   zao,\               ny        @@

--

[S] |                   :                   !                   :                   ||
[A] |                   :                   !                   :                   ||
[T] |m                  :m            .,f   !s                  :l            .,s   ||
[B] |d                  :d            .,r   !m                  :f            .,m   ||

[1] ta_                 ny             i_   zao                 fa             Je_  @@

--

[S] |                   :                   !                   :         .s-1      ||
[A] |                   :                   !                   :         .m-1      ||
[T] |s                  :-                  !m                  :         .d        ||
[B] |m                  :-                  !d                  :         .d-1      ||

[1] ho@2_                                   vah                 ~         ma_       @@

--

[S] |d                  :d            .,d   !d                  :         .d        ||
[A] |m-1                :m-1          .,m-1 !m-1                :         .s-1      ||
[T] |d                  :d            .,d   !d                  :         .d        ||
[B] |d-1                :d-1          .,d-1 !d-1                :         .m-1      ||

[1] na_                 mpy            a_   nao,\               ~         ma_       @@

--

[S] |r                  :r            .,r   !r                  :r                  ||
[A] |s-1                :s-1          .,s-1 !s-1                :s-1                ||
[T] |t-1                :t-1          .,t-1 !t-1                :t-1                ||
[B] |s-1                :s-1          .,s-1 !s-1                :s-1                ||

[1] na_                 mpy            a_   nao,                ma_                 @@

--

[S] |m                  :m            .,m   !m                  :r                  ||
[A] |s-1                :s-1          .,s-1 !s-1                :l-1                ||
[T] |d                  :d            .,d   !d                  :f                  ||
[B] |d                  :d            .,d   !d                  :f-1                ||

[1] na_                 mpy            a_   nao,                ka                  @@

--

[S] |d            .,d   :t-1          .,t-1 !d                  :                   ||
[A] |s-1          .,s-1 :s-1          .,s-1 !s-1                :                   ||
[T] |m            .,m   :f            .,f   !m                  :                   ||
[B] |s-1          .,s-1 :s-1          .,s-1 !d-1                :                   ||

[1] a_             za   ki_            vy   fo.\                ~                   @@

--

[S] |                   :                   !                   :                   ||
[A] |                   :                   !                   :                   ||
[T] |                   :                   !                   :                   ||
[B] |`d           .,s`  :m-1          .,s-1 !d                  :d                  ||

[1] A@2_                za             ma_  ngi_                na,                 @@

--

[S] |                   :                   !                   :                   ||
[A] |                   :                   !                   :                   ||
[T] |`m       .r`       :d        .r        !m                  :m                  ||
[B] |                   :                   !                   :                   ||

[1] a@2_                za        ma_       ngi_                na                  @@

--

[S] |s                  :s            .,s   !l                  :s            .,f   ||
[A] |d                  :d            .,d   !d                  :d            .,d   ||
[T] |m                  :m            .,m   !f                  :m            .,l   ||
[B] |d                  :d            .,d   !d                  :d            .,f   ||

[1] fa                  ny             fa_  ne_                 va             no   @@

--

[$] touches={frm:4}

[S] |m                  :r                  !d                  :d                  ||
[A] |d                  :s-1                !s-1                :s-1                ||
[T] |s                  :f                  !m                  :m                  ||
[B] |s-1                :s-1                !d-1                :d-1                ||

[1] a_                  tsa_                nga_                no.\                @@

--

[$] time={3,4}

[S] |                   :                   ||
[A] |                   :                   ||
[T] |                   :                   ||
[B] |                   :                   ||

[1] ~                   ~                   @@

--

[$] mark={S}

[S] :s-1          .,s-1 ||
[A] :m-1          .,m-1 ||
[T] :d-1          .,d   ||
[B] :d-1          .,d-1 ||

[1] Ha_            le_  @@

--

[S] |s-1                :s-1                :s-1          .,s-1 ||
[A] |m-1                :m-1                :m-1          .,m-1 ||
[T] |d                  :d                  :d            .,d   ||
[B] |d-1                :d-1                :d-1          .,d-1 ||

[1] lo_                 ia!                 Ha_            le_  @@

--

[S] |d                  :d                  :d            .,d   ||
[A] |s-1                :s-1                :s-1          .,s-1 ||
[T] |d                  :d                  :d            .,d   ||
[B] |m-1                :m-1                :m-1          .,m   ||

[1] lo_                 ia!                 Ha_            le_  @@

--

[S] |r                  :d                  :r                  ||
[A] |f-1                :m-1                :s-1                ||
[T] |t-1                :d                  :t-1                ||
[B] |s-1                :s-1                :s-1                ||

[1] lo_                 ia!                 A_                  @@

--

[$] dynamics={f:1}

[S] |m                  :-                  :l            .,l   ||
[A] |s-1                :-                  :d            .,d   ||
[T] |d                  :-                  :f            .,f   ||
[B] |d                  :-                  :f            .,f   ||

[1] mena@2\                                 Ha_            le_  @@

--

[S] |s                  :m                  :s            .,s   ||
[A] |d                  :d                  :d            .,d   ||
[T] |m                  :d                  :m            .,m   ||
[B] |m                  :d                  :m            .,m   ||

[1] lo_                 ia!                 Ha_            le_  @@

--

[S] |f                  :r                  :f            .,f   ||
[A] |t-1                :t-1                :t-1          .,t-1 ||
[T] |s                  :s                  :s            .,s   ||
[B] |r                  :s-1                :r            .,r   ||

[1] lo_                 ia!                 Ha_            le_  @@

--

[S] |m                  :r                  :t-1                ||
[A] |d                  :l-1                :s-1                ||
[T] |s                  :f                  :r                  ||
[B] |d                  :f-1                :s-1                ||

[1] lo_                 ia!                 A_                  @@

--

[S] |d                  :-                  :-                  ||
[A] |s-1                :-                  :s-1                ||
[T] |m                  :-                  :m                  ||
[B] |d-1                :-                  :d-1                ||

[1] mena@2\                                 A_                  @@

--

[S] |-                  :-                  :-                  ||
[A] |l-1                :-            .,s   :l-1      .s-1      ||
[T] |f                  :-            .,m   :f        .m        ||
[B] |f-1                :-        .d-1      :f-1      .d        ||

[1] mena,               A_                  mena                @@

--

[S] |-                  :                   :                   ||
[A] |l-1                :                   :                   ||
[T] |f                  :                   :                   ||
[B] |f-1                :                   :                   ||

[1] Amena               ~                   ~                   @@

--

[$] touches={frm}

[S] |s                  :-                  :t-1                ||
[A] |t-1                :-                  :s-1                ||
[T] |r                  :-                  :f                  ||
[B] |s-1                :-                  :-                  ||

[1] A@3_                                                        @@

--

[$] tempo={rit} | jump={DS}

[S] |d                  :-                  :                   ||
[A] |s-1                :-                  :                   ||
[T] |m                  :-                  :                   ||
[B] |d-1                :-                  :                   ||

[1] mena.@2                                 ~
```

## Example 2

```voxalfa
;; @version 0.1.0-alpha

[#] title="Ry Tanindrazanay Malala ô!"
[#] author={"Norbert Raharisoa","Pasteur Rahajason"}
[#] description="The national anthem of Madagascar"
[#] verses={1}
[#] release={1958}
[#] language="mg"
[#] tags={"anthem","madagascar"}

[$] key={Bb} | time={4,4} | tempo={110} | voices={S,A,T,B}

---

[S] :s-1                ||
[A] :s-1                ||
[T] :s-1                ||
[B] :s-1                ||

[1] Ry                  @@

--

[S] |d                  :`d           .,d`  !d            .,s-1 :d            .,r   ||
[A] |m-1                :`m-1         .,m-1`!m-1          .,m-1 :m-1          .,s-1 ||
[T] |s-1                :`s-1         .,s-1`!s-1          .,s-1 :s-1          .,s-1 ||
[B] |d-1                :`d-1         .,d-1`!d-1          .,d-1 :d-1          .,d-1 ||

[1] Ta_                 nin@2_              dra_           za_  nay            Ma_  @@

--

[S] |m                  :-        .r        !d                  :s-1                ||
[A] |s-1                :-        .f-1      !m-1                :m-1                ||
[T] |s-1                :-        .s-1      !s-1                :d                  ||
[B] |d-1                :-        .d-1      !d-1                :s-1                ||

[1] la@2_                         la        ô!\                 Ry                  @@

--

[S] |l                  :`l           .,l`  !`t   ,d+1  ,t`     :l            .,t   ||
[A] |f-1                :`f-1         .,f-1`!f-1      .f-1      :f-1          .,f-1 ||
[T] |d                  :`d           .,r`  !m        .m        :r            .,r   ||
[B] |f-1          .,m-1 :`r-1         .,r-1`!s-1      .s-1      :s-1          .,s-1 ||

[1] Ma_                 da@2_               gasi_               ka_            ra   @@

--

[S] |d                  :-                  !-                  :s-1                ||
[A] |m-1                :-                  !-                  :s-1                ||

[1] soa@3\                                                      Ny                  @@

++

[T] |d                  :-                  !-                  :                   ||

[1] soa@3\                                                      ~                   @@

++

[B] |d                  :-                  !-                  :                   ||

[1] soa@3\                                                      ~                   @@

--

[S] |m                  :m            .,m   !m            .,r   :m            .,f   ||
[A] |d                  :d            .,d   !d            .,t-1 :d            .,f   ||

[1] fi_                 tia_           va_  nay            A_   nao            tsy  @@

++

[T] |d                  :-                  !d                  :-                  ||

[1] Ny@2                                    fi@2_                                   @@

++

[B] |d                  :t-1                !l-1                :s-1          .,f-1 ||

[1] Ny                  fi_                 ti_                 a_             va_  @@

--

[S] |`s                 :-        .f`       !m            .,s   ||
[A] |`m                 :-        .r`       !d            .,s   ||

[1] mia@3_                                  la,\           Fa   @@

++

[T] |d                  :-                  !d                  ||

[1] tia@2_                                  va_                 @@

++

[B] |m-1                :r-1                !d-1                ||

[1] nay                 A_                  nao                 @@

--

[$] key={F}

[S] :t            .,l   ||
[A] :d            .,d   ||

[1] ho             A_   @@

++

[T] :ri                 ||

[1] nay                 @@

++

[B] :fi                 ||

[1] tsy                 @@

--

[S] |s                  :m            .,f   !s            .,s   :f            .,r   ||
[A] |d                  :d            .,d   !t-1          .,t-1 :t-1          .,t-1 ||

[1] nao                 ho             a_   nao            do_  ria            to_  @@

++

[T] |m                  :m                  !r                  :`s           .,f`  ||

[1] A_                  nao                 tsy                 mia@2_              @@

++

[B] |s-1                :-                  !s-1                :-                  ||

[1] mi@2_                                   a@2_                                    @@

--

[S] |d                  :-                  !-                  ||
[A] |d                  :-                  !-                  ||

[1] koa@3\                                                      @@

++

[T] |m                  :-                  !-                  ||

[1] la@3\                                                       @@

++

[B] |d-1                :-                  !-                  ||

[1] la@3\                                                       @@

--

[$] mark={S} | key={Bb}

[S] :s-1                ||
[A] :s-1                ||

[1] Ta_                 @@

++

[T] :                   ||
[B] :                   ||

[1] ~                   @@

--

[S] |r                  :-        .r        !r            .,di  :r            .,m   ||
[A] |f-1                :-        .f-1      !f-1          .,m-1 :f-1          .,s-1 ||

[1] hi@2_                         o_        nao            Ry   Za_            na_  @@

++

[T] |t-1                :-                  !-                  :r                  ||

[1] Za@3_                                                       na_                 @@

++

[B] |s-1                :-                  !-                  :`s   ,l    ,t`     ||

[1] Za@3_                                                       na@3_               @@

--

[S] |d                  :-                  !s-1                :s-1                ||
[A] |m-1                :-                  !m-1                :s-1                ||

[1] ha@2_                                   ry\                 'Ty                 @@

++

[T] |d                  :-                  !d                  :                   ||

[1] ha@2_                                   ry@2\                                   @@

++

[B] |d                  :-                  !d                  :                   ||

[1] ha@2_                                   ry@2\                                   @@

--

[S] |f                  :-        .f        !f            .,m   :f            .,s   ||
[A] |r                  :-        .r        !r            .,di  :r            .,m   ||

[1] no@2_                         sin-_     dra_           za_  nay            i_   @@

++

[T] |t-1                :-                  !d                  :r                  ||

[1] 'Ty@2                                   no                  si_                 @@

++

[B] |s-1                :-                  !l-1                :t-1                ||

[1] 'Ty@2                                   no                  sin_                @@

--

[S] |m                  :-                  !-                  :d                  ||
[A] |d                  :-                  !-                  :d                  ||

[1] ty@3\                                                       Hi_                 @@

++

[T] |d                  :-                  !-                  :                   ||

[1] nay@3\                                                      ~                   @@

++

[B] |d                  :t-1                !l                  :s                  ||

[1] dra_                za_                 nay\                Hi_                 @@

--

[S] |l                  :-        .s        !f            .,r   :m            .,f   ||
[A] |d                  :di                 !r            .,r   :d            .,d   ||

[1] a_                  da_                 na             sy   ho             fi_  @@

++

[T] |l                  :-                  !-                  :l-1          .,l-1 ||

[1] He@3                                                        sam_           ba_  @@

++

[B] |f-1                :m-1                !r-1          .,f-1 :m-1          .,r-1 ||

[1] a_                  da_                 na             sy   ho             fi_  @@

--

[S] |s                  :-        .f        !m                  :m                  ||
[A] |d                  :t-1                !d                  :ta-1               ||

[1] na_                 ri_                 tra\                He                  @@

++

[T] |`s-1               :-                  !s`                 :di                 ||

[1] tra@3                                                       to_                 @@

++

[B] |m-1                :r-1                !d-1                :s-1                ||

[1] na_                 ri_                 tra\                He                  @@

--

[S] |r            .,di  :r            .,m   !f                  :l-1          .,t-1 ||
[A] |l-1          .,l-1 :s-1          .,s-1 !f-1                :f-1          .,f-1 ||

[1] sam_           ba_  tra            to_  koa                 i_             za_  @@

++

[T] |`s                 :-        .di       !r`                 :r            .,r   ||

[1] koa@4                                                       i_             za_  @@

++

[B] |f-1                :m-1                !r-1                :s-1                ||

[1] sam_                ba_                 (tra i)_            za_                 @@

--

[$] ending={1} | jump={DS}

[S] |d                  :-                  !-                  :s-1                ||
[A] |m                  :-                  !-                  :s-1                ||
[T] |d                  :-                  !-                  :ta                 ||

[1] hay@4

++

[B] |d-1                :-                  !-                  :                   ||

[1] hay@3

--

[$] ending={2} | label="Fin"

[S] |d                  :-                  !-                  :-                  ||
[A] |d                  :-                  !-                  :-                  ||
[T] |d                  :-                  !-                  :-                  ||
[B] |d                  :-                  !-                  :-                  ||

[1] hay@4
```
