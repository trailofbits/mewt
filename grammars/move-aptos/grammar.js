/// <reference types="tree-sitter-cli/dsl" />
// @ts-check
//
// Tree-sitter grammar for Move on Aptos (Move 2.x)
//
// Canonical reference: the Move compiler v2 parser
//   https://github.com/move-language/move-on-aptos/blob/main/language/move-compiler/src/parser/syntax.rs
//
// Move Book: https://aptos.dev/en/build/smart-contracts/book

// ─── Helpers ──────────────────────────────────────────────────────────────────

const sepBy1 = (sep, rule) => seq(rule, repeat(seq(sep, rule)));
const sepBy = (sep, rule) => optional(sepBy1(sep, rule));
const commaSep = rule => seq(sepBy(',', rule), optional(','));
const commaSep1 = rule => seq(sepBy1(',', rule), optional(','));

// ─── Precedence ───────────────────────────────────────────────────────────────
// Based on the Move compiler's BinOp precedence.

const PREC = {
    ASSIGN: 1,
    // spec-only: ==> and <==>
    IFF: 2,
    IMPLIES: 2,
    OR: 3, // ||
    AND: 4, // &&
    EQ: 5, // == != < > <= >=
    RANGE: 6, // .. (spec only)
    BITOR: 7, // |
    XOR: 8, // ^
    BITAND: 9, // &
    SHIFT: 10, // << >>
    ADD: 11, // + -
    MUL: 12, // * / %
    UNARY: 13, // ! & &mut * move copy
    AS: 14, // as (type cast)
    FIELD: 15, // . (field access, method call)
    CALL: 16, // function calls
};

// ─── Grammar ──────────────────────────────────────────────────────────────────

module.exports = grammar({
    name: 'move_on_aptos',

    word: $ => $.identifier,

    extras: $ => [$.line_comment, $.block_comment, /\s/],

    externals: $ => [
        $._block_doc_comment_marker,
        $._block_comment_content,
        $._doc_line_comment,
        $._error_sentinel,
    ],

    inline: $ => [
        $._module_member,
        $._attribute_value,
        $._sequence_item,
        $._spec_function,
        $._field_identifier,
        $._modifier,
        $._bind,
        $._literal_value,
        $._spec_block_member,
        $._match_pattern,
    ],

    supertypes: $ => [],

    conflicts: $ => [
        // 'exists' and 'forall' are both spec quantifier keywords and valid identifiers
        [$._leading_name_access, $.quantifier_expression],
        // name_access_chain followed by < is ambiguous: comparison (binary_expression)
        // vs start of type_arguments in call_expression / pack_expression.
        // GLR forks here; prec.dynamic(2) on type_arguments paths makes them win
        // when both parses succeed (e.g. exists<T>(addr)).
        [$.name_expression, $.call_expression, $.pack_expression, $.generic_name_expression],
        // In is_expression: name_access_chain < could be type_arguments or comparison
        [$._is_variant],
        // In is_expression: | could be variant separator or binary OR
        [$.is_variant_list],
        // spec fun name(params): type { } could be _spec_function or _spec_block_target
        [$._spec_block_target, $._spec_function_signature],
        // spec name ( could be name_access_chain(identifier, ...) or spec_function_signature(identifier, params)
        [$._leading_name_access, $._spec_function_signature],
        // After dot_expression, < is ambiguous: reduce dot_expression to _expression_term
        // (then binary <) vs keep dot_expression for method_call_with_type_args (shift <).
        [$._expression_term, $.method_call_with_type_args],
        // 'friend package' could be visibility modifier or friend declaration with 'package' as name
        [$.friend_declaration, $._visibility],
        // In quantifier trigger: name_access_chain { could be pack_expression or trigger start
        [$.name_expression, $.pack_expression],
        [$.generic_name_expression, $.pack_expression],
        [$.generic_name_expression, $.call_expression],
        // Primitive type keywords (u8, address, bool, etc.) can also be module names
        // in access paths (e.g. i64::I64 in decompiled bytecode)
        [$.primitive_type, $._leading_name_access],
        // 'match' can be a keyword or a module name in access paths
        [$._leading_name_access, $.match_expression],
    ],

    precedences: _ => [],

    rules: {
        // ═══════════════════════════════════════════════════════════════════════════
        // Top-level
        // ═══════════════════════════════════════════════════════════════════════════

        source_file: $ =>
            repeat(
                choice($.module_declaration, $.script_declaration, $.address_block, $.spec_block)
            ),

        // ═══════════════════════════════════════════════════════════════════════════
        // Address blocks (legacy syntax)
        // ═══════════════════════════════════════════════════════════════════════════

        address_block: $ =>
            seq(
                optional($.attributes),
                'address',
                field('address', $._leading_name_access),
                '{',
                repeat($.module_declaration),
                '}'
            ),

        // ═══════════════════════════════════════════════════════════════════════════
        // Module declaration
        // ═══════════════════════════════════════════════════════════════════════════

        module_declaration: $ =>
            seq(
                optional($.attributes),
                'module',
                field('name', choice($.module_identity, $.identifier)),
                field('body', $.module_body)
            ),

        module_identity: $ =>
            seq(field('address', $._leading_name_access), '::', field('module', $.identifier)),

        module_body: $ => seq('{', repeat($._module_member), '}'),

        _module_member: $ =>
            choice(
                $.use_declaration,
                $.friend_declaration,
                $.constant_declaration,
                $.struct_declaration,
                $.enum_declaration,
                $.function_declaration,
                $.spec_block,
                // Module-level invariant: shorthand for spec module { invariant ... }
                $.spec_invariant
            ),

        // ═══════════════════════════════════════════════════════════════════════════
        // Script declaration
        // ═══════════════════════════════════════════════════════════════════════════

        script_declaration: $ =>
            seq(
                optional($.attributes),
                'script',
                '{',
                repeat(choice($.use_declaration, $.constant_declaration)),
                $.function_declaration,
                repeat($.spec_block),
                '}'
            ),

        // ═══════════════════════════════════════════════════════════════════════════
        // Use declarations
        // ═══════════════════════════════════════════════════════════════════════════

        use_declaration: $ =>
            seq(
                optional($.attributes),
                'use',
                choice($.use_module, $.use_module_member, $.use_module_members),
                ';'
            ),

        use_module: $ => seq($.module_identity, optional($.use_alias)),

        use_module_member: $ => seq($.module_identity, '::', $.use_member),

        use_module_members: $ => seq($.module_identity, '::', '{', commaSep($.use_member), '}'),

        use_member: $ => seq(field('member', $.identifier), optional($.use_alias)),

        use_alias: $ => seq('as', field('alias', $.identifier)),

        // ═══════════════════════════════════════════════════════════════════════════
        // Friend declaration
        // ═══════════════════════════════════════════════════════════════════════════

        friend_declaration: $ => seq(optional($.attributes), 'friend', $.name_access_chain, ';'),

        // ═══════════════════════════════════════════════════════════════════════════
        // Constant declaration
        // ═══════════════════════════════════════════════════════════════════════════

        constant_declaration: $ =>
            seq(
                optional($.attributes),
                'const',
                field('name', $.identifier),
                ':',
                field('type', $._type),
                '=',
                field('value', $._expression),
                ';'
            ),

        // ═══════════════════════════════════════════════════════════════════════════
        // Struct declaration
        // ═══════════════════════════════════════════════════════════════════════════

        struct_declaration: $ =>
            choice($._native_struct_declaration, $._regular_struct_declaration),

        _native_struct_declaration: $ =>
            seq(
                optional($.attributes),
                repeat($._modifier),
                'native',
                'struct',
                field('name', $.identifier),
                optional(field('type_parameters', $.type_parameters)),
                optional(field('abilities', $.ability_declarations)),
                ';'
            ),

        _regular_struct_declaration: $ =>
            seq(
                optional($.attributes),
                repeat($._modifier),
                'struct',
                field('name', $.identifier),
                optional(field('type_parameters', $.type_parameters)),
                choice(
                    // Named fields: struct Foo has copy { x: u64, y: bool }
                    // Also handles post-abilities: struct Foo { x: u64 } has copy, drop;
                    seq(
                        optional(field('abilities', $.ability_declarations)),
                        field('fields', $.struct_fields),
                        optional(seq(field('post_abilities', $.ability_declarations), ';'))
                    ),
                    // Positional fields: struct Pair(u64, u8) has copy, drop;
                    seq(
                        field('fields', $.positional_fields),
                        optional(field('abilities', $.ability_declarations)),
                        ';'
                    ),
                    // No fields (type tag): struct TypeTag has copy, drop;
                    seq(optional(field('abilities', $.ability_declarations)), ';')
                )
            ),

        struct_fields: $ => seq('{', commaSep($.field_declaration), '}'),

        field_declaration: $ =>
            seq(field('name', $._field_identifier), ':', field('type', $._type)),

        // Field names may include reserved words like 'for' that are valid field identifiers
        _field_identifier: $ => choice($.identifier, alias('for', $.identifier)),

        positional_fields: $ => seq('(', commaSep($._type), ')'),

        ability_declarations: $ => seq('has', commaSep1($.ability)),

        ability: _ => choice('copy', 'drop', 'store', 'key'),

        // ═══════════════════════════════════════════════════════════════════════════
        // Enum declaration
        // ═══════════════════════════════════════════════════════════════════════════

        enum_declaration: $ =>
            seq(
                optional($.attributes),
                repeat($._modifier),
                'enum',
                field('name', $.identifier),
                optional(field('type_parameters', $.type_parameters)),
                optional(field('abilities', $.ability_declarations)),
                '{',
                repeat(seq($.enum_variant, optional(','))),
                '}',
                // Abilities can also appear after the closing brace: } has drop, copy;
                optional(seq(field('post_abilities', $.ability_declarations), ';'))
            ),

        enum_variant: $ =>
            seq(
                field('name', $.identifier),
                optional(
                    choice(field('fields', $.struct_fields), field('fields', $.positional_fields))
                )
            ),

        // ═══════════════════════════════════════════════════════════════════════════
        // Function declaration
        // ═══════════════════════════════════════════════════════════════════════════

        function_declaration: $ =>
            choice($._native_function_declaration, $._regular_function_declaration),

        _native_function_declaration: $ =>
            seq(
                optional($.attributes),
                repeat($._modifier),
                'native',
                repeat($._modifier),
                'fun',
                field('name', choice($.identifier, alias('for', $.identifier))),
                optional(field('type_parameters', $.type_parameters)),
                field('parameters', $.function_parameters),
                optional(seq(':', field('return_type', $._type))),
                // TODO: The Move language spec only allows a single acquires clause
                // with comma-separated resources, but the bytecode decompiler emits
                // multiple acquires clauses on separate lines. We allow repeat here
                // to handle decompiled code.
                repeat(choice(field('acquires', $.acquires_clause), $.access_specifier)),
                ';'
            ),

        _regular_function_declaration: $ =>
            seq(
                optional($.attributes),
                repeat($._modifier),
                'fun',
                field('name', choice($.identifier, alias('for', $.identifier))),
                optional(field('type_parameters', $.type_parameters)),
                field('parameters', $.function_parameters),
                optional(seq(':', field('return_type', $._type))),
                // TODO: The Move language spec only allows a single acquires clause
                // with comma-separated resources, but the bytecode decompiler emits
                // multiple acquires clauses on separate lines. We allow repeat here
                // to handle decompiled code.
                repeat(choice(field('acquires', $.acquires_clause), $.access_specifier)),
                field('body', $.block)
            ),

        _modifier: $ => choice($._visibility, $.entry_modifier, $.inline_modifier),

        _visibility: $ =>
            choice(
                'public',
                seq('public', '(', 'friend', ')'),
                seq('public', '(', 'package', ')'),
                seq('public', '(', 'script', ')'),
                'friend',
                'package'
            ),

        entry_modifier: _ => 'entry',
        inline_modifier: _ => 'inline',

        function_parameters: $ => seq('(', commaSep($.function_parameter), ')'),

        function_parameter: $ => seq(field('name', $.identifier), ':', field('type', $._type)),

        acquires_clause: $ => seq('acquires', commaSep1($.name_access_chain)),

        // Access specifiers (Move 2.5+): reads R, writes T, pure, !reads *(0x42)
        access_specifier: $ =>
            choice(
                'pure',
                seq(optional('!'), choice('reads', 'writes'), commaSep1($.access_specifier_arg))
            ),

        access_specifier_arg: $ =>
            choice(
                // Wildcard: * or *(expr)
                seq('*', optional(seq('(', choice('*', $._expression), ')'))),
                // Named path with optional wildcards: name, 0x42::m::S, 0x42::*::*
                seq(
                    $._leading_name_access,
                    repeat(seq('::', choice($.identifier, '*'))),
                    optional($.type_arguments),
                    optional(seq('(', choice('*', $._expression), ')'))
                )
            ),

        // ═══════════════════════════════════════════════════════════════════════════
        // Attributes
        // ═══════════════════════════════════════════════════════════════════════════

        attributes: $ => repeat1(seq('#', '[', commaSep($.attribute), ']')),

        attribute: $ =>
            seq(
                $.name_access_chain,
                optional(choice(seq('=', $._attribute_value), seq('(', commaSep($.attribute), ')')))
            ),

        _attribute_value: $ =>
            choice(
                $._literal_value,
                $.name_access_chain,
                // Bare address module paths: 0000...0001::module (no 0x prefix)
                seq($.num_literal, '::', $.identifier)
            ),

        // ═══════════════════════════════════════════════════════════════════════════
        // Type parameters (generics)
        // ═══════════════════════════════════════════════════════════════════════════

        type_parameters: $ => seq('<', commaSep1($.type_parameter), '>'),

        type_parameter: $ =>
            seq(
                optional('phantom'),
                field('name', $.identifier),
                optional(seq(':', field('constraints', $.ability_constraints)))
            ),

        ability_constraints: $ => prec.left(sepBy1('+', $.ability)),

        type_arguments: $ => seq('<', commaSep1($._type), '>'),

        // ═══════════════════════════════════════════════════════════════════════════
        // Types
        // ═══════════════════════════════════════════════════════════════════════════

        _type: $ =>
            choice(
                $.primitive_type,
                $.ref_type,
                $.tuple_type,
                $.apply_type,
                $.function_type,
                $.unit_type
            ),

        primitive_type: _ =>
            choice(
                'u8',
                'u16',
                'u32',
                'u64',
                'u128',
                'u256',
                'bool',
                'address',
                'signer',
                // Signed integers (Move 2.3)
                'i8',
                'i16',
                'i32',
                'i64',
                'i128',
                'i256'
            ),

        ref_type: $ => seq(choice('&', '&mut'), $._type),

        tuple_type: $ => seq('(', commaSep1($._type), ')'),

        unit_type: _ => seq('(', ')'),

        // Apply type: Foo, Foo<T>, vector<u8>, module::Struct<T>
        apply_type: $ => prec.left(seq($.name_access_chain, optional($.type_arguments))),

        // Function type: |u64, bool| u64 has copy + drop
        // Also handles: |u64| u64 with store+copy
        // Also handles empty params: || u64 or | | u64
        function_type: $ =>
            prec.right(
                choice(
                    seq(
                        '|',
                        commaSep1($._type),
                        '|',
                        optional($._type),
                        optional(seq(choice('has', 'with'), $.ability_constraints))
                    ),
                    seq(
                        '||',
                        optional($._type),
                        optional(seq(choice('has', 'with'), $.ability_constraints))
                    ),
                    // Zero-param with space between pipes: | | u64
                    seq(
                        '|',
                        '|',
                        optional($._type),
                        optional(seq(choice('has', 'with'), $.ability_constraints))
                    )
                )
            ),

        // ═══════════════════════════════════════════════════════════════════════════
        // Name access chains (qualified paths)
        // ═══════════════════════════════════════════════════════════════════════════
        // These represent: name, addr::module, addr::module::member, addr::module::Type::Variant

        _leading_name_access: $ =>
            choice(
                $.identifier,
                $.numerical_address,
                // Contextual keywords that are also valid identifiers in expression context
                alias('vector', $.identifier), // vector[..] literal, but also vector::length()
                alias('exists', $.identifier), // spec quantifier, but also exists<T>(addr) builtin
                alias('forall', $.identifier), // spec quantifier, but may appear as name
                alias('choose', $.identifier), // spec quantifier, but may appear as name
                alias('package', $.identifier), // visibility modifier, but also valid as named address
                alias('for', $.identifier), // for-loop keyword, but also valid as name in some contexts
                alias('match', $.identifier), // match keyword, but also valid as module name
                // Primitive type keywords that can appear as module names in access paths
                // (e.g. i64::I64, u16::U16 — common in decompiled bytecode)
                alias('u8', $.identifier),
                alias('u16', $.identifier),
                alias('u32', $.identifier),
                alias('u64', $.identifier),
                alias('u128', $.identifier),
                alias('u256', $.identifier),
                alias('bool', $.identifier),
                alias('address', $.identifier),
                alias('signer', $.identifier),
                alias('i8', $.identifier),
                alias('i16', $.identifier),
                alias('i32', $.identifier),
                alias('i64', $.identifier),
                alias('i128', $.identifier),
                alias('i256', $.identifier)
            ),

        numerical_address: _ => /0x[a-fA-F0-9]+/,

        name_access_chain: $ =>
            prec.left(seq($._leading_name_access, repeat(seq('::', $.identifier)))),

        // module_name is just an identifier - use $.identifier directly where needed

        // ═══════════════════════════════════════════════════════════════════════════
        // Literals
        // ═══════════════════════════════════════════════════════════════════════════

        _literal_value: $ =>
            choice(
                $.address_literal,
                $.num_literal,
                $.bool_literal,
                $.byte_string_literal,
                $.hex_string_literal
            ),

        address_literal: $ => seq('@', choice($._leading_name_access, $.num_literal)),

        num_literal: _ =>
            token(
                choice(
                    // Decimal with optional type suffix
                    /[0-9][0-9_]*(u8|u16|u32|u64|u128|u256|i8|i16|i32|i64|i128|i256)?/,
                    // Hex with optional type suffix
                    /0[xX][a-fA-F0-9][a-fA-F0-9_]*(u8|u16|u32|u64|u128|u256|i8|i16|i32|i64|i128|i256)?/
                )
            ),

        bool_literal: _ => choice('true', 'false'),

        byte_string_literal: _ =>
            token(seq('b"', repeat(choice(/\\[nrt0\\"]/, /\\x[0-9a-fA-F]{2}/, /[^\\"]/)), '"')),

        hex_string_literal: _ => token(seq('x"', /[0-9a-fA-F]*/, '"')),

        // ═══════════════════════════════════════════════════════════════════════════
        // Expressions
        // ═══════════════════════════════════════════════════════════════════════════

        _expression: $ =>
            choice(
                $.lambda_expression,
                $.if_expression,
                $.while_expression,
                $.loop_expression,
                $.for_expression,
                $.match_expression,
                $.return_expression,
                $.abort_expression,
                $.assign_expression,
                $._unary_expression,
                $.binary_expression,
                $.quantifier_expression
            ),

        // ─── Lambda ───────────────────────────────────────────────────────────────

        lambda_expression: $ =>
            prec.right(
                seq(
                    field('parameters', $.lambda_parameters),
                    field('body', $._expression),
                    // Lambda can have a trailing spec block: |x| expr spec { ensures ... }
                    optional(field('spec', $.spec_block))
                )
            ),

        lambda_parameters: $ =>
            choice(seq('|', commaSep1($.lambda_parameter), '|'), '||', seq('|', '|')),

        lambda_parameter: $ =>
            seq(field('bind', $._bind), optional(seq(':', field('type', $._type)))),

        // ─── Control flow ─────────────────────────────────────────────────────────

        if_expression: $ =>
            prec.right(
                seq(
                    'if',
                    '(',
                    field('condition', $._expression),
                    ')',
                    field('consequence', $._expression),
                    optional(seq('else', field('alternative', $._expression)))
                )
            ),

        while_expression: $ =>
            prec.right(
                seq(
                    optional($.loop_label),
                    'while',
                    '(',
                    field('condition', $._expression),
                    ')',
                    field('body', $._expression),
                    // Optional inline spec block: while (cond) { body } spec { invariant ...; }
                    optional(field('spec', $.spec_block))
                )
            ),

        loop_expression: $ =>
            prec.right(
                seq(
                    optional($.loop_label),
                    'loop',
                    field('body', $._expression),
                    optional(field('spec', $.spec_block))
                )
            ),

        for_expression: $ =>
            prec(
                1,
                seq(
                    'for',
                    '(',
                    field('iterator', $.identifier),
                    'in',
                    field('range', $.range_expression),
                    ')',
                    field('body', $._expression)
                )
            ),

        range_expression: $ =>
            prec(
                PREC.RANGE + 1,
                seq(field('lower', $._expression), '..', field('upper', $._expression))
            ),

        loop_label: _ => token(seq("'", /[a-zA-Z_][a-zA-Z0-9_]*/, ':')),

        match_expression: $ =>
            seq('match', '(', field('subject', $._expression), ')', '{', repeat($.match_arm), '}'),

        match_arm: $ =>
            seq(
                field('pattern', $._match_pattern),
                // Guard with or without parens: `if cond` or `if (cond)`
                // Using _expression handles both: (cond) is a parenthesized_expression
                optional(seq('if', field('guard', $._expression))),
                '=>',
                field('body', $._expression),
                optional(',')
            ),

        _match_pattern: $ =>
            choice(
                $.name_expression,
                $.struct_pattern,
                $.positional_pattern,
                $.tuple_pattern,
                $.or_pattern,
                '_',
                $._literal_value
            ),

        struct_pattern: $ =>
            seq(
                $.name_access_chain,
                optional($.type_arguments),
                '{',
                commaSep(choice($.field_pattern, '..')),
                '}'
            ),

        field_pattern: $ =>
            seq(
                field('field', $._field_identifier),
                optional(seq(':', field('bind', $._match_pattern)))
            ),

        positional_pattern: $ =>
            seq(
                $.name_access_chain,
                optional($.type_arguments),
                '(',
                commaSep(choice($._match_pattern, '..')),
                ')'
            ),

        // Tuple pattern: (Pat1, Pat2) — no name_access_chain prefix
        tuple_pattern: $ => seq('(', commaSep(choice($._match_pattern, '..')), ')'),

        or_pattern: $ => prec.left(seq($._match_pattern, '|', $._match_pattern)),

        // ─── Return, abort, break, continue ───────────────────────────────────────

        return_expression: $ => prec.right(seq('return', optional($._expression))),

        abort_expression: $ => prec.right(seq('abort', $._expression)),

        break_expression: $ =>
            prec.right(seq('break', optional($.label_name), optional($._expression))),

        continue_expression: $ => seq('continue', optional($.label_name)),

        label_name: _ => token(seq("'", /[a-zA-Z_][a-zA-Z0-9_]*/)),

        // ─── Assignment ───────────────────────────────────────────────────────────

        assign_expression: $ =>
            prec.right(
                PREC.ASSIGN,
                seq(
                    field('lhs', $._unary_expression),
                    field(
                        'op',
                        choice('=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=')
                    ),
                    field('rhs', $._expression)
                )
            ),

        // ─── Binary expressions ───────────────────────────────────────────────────

        binary_expression: $ => {
            const table = [
                [PREC.IMPLIES, '==>'],
                [PREC.IFF, '<==>'],
                [PREC.OR, '||'],
                [PREC.AND, '&&'],
                [PREC.EQ, '=='],
                [PREC.EQ, '!='],
                [PREC.EQ, '<'],
                [PREC.EQ, '>'],
                [PREC.EQ, '<='],
                [PREC.EQ, '>='],
                [PREC.RANGE, '..'],
                [PREC.BITOR, '|'],
                [PREC.XOR, '^'],
                [PREC.BITAND, '&'],
                [PREC.SHIFT, '<<'],
                [PREC.SHIFT, '>>'],
                [PREC.ADD, '+'],
                [PREC.ADD, '-'],
                [PREC.MUL, '*'],
                [PREC.MUL, '/'],
                [PREC.MUL, '%'],
            ];

            return choice(
                ...table.map(([precedence, operator]) =>
                    prec.left(
                        precedence,
                        seq(
                            field('lhs', $._expression),
                            field('operator', operator),
                            field('rhs', $._expression)
                        )
                    )
                )
            );
        },

        // ─── Unary expressions ────────────────────────────────────────────────────

        _unary_expression: $ =>
            choice(
                $.not_expression,
                $.negate_expression,
                $.borrow_expression,
                $.dereference_expression,
                $.move_expression,
                $.copy_expression,
                $._expression_term
            ),

        not_expression: $ => prec(PREC.UNARY, seq('!', $._unary_expression)),

        negate_expression: $ => prec(PREC.UNARY, seq('-', $._unary_expression)),

        borrow_expression: $ => prec(PREC.UNARY, seq(choice('&', '&mut'), $._unary_expression)),

        dereference_expression: $ => prec(PREC.UNARY, seq('*', $._unary_expression)),

        move_expression: $ => prec(PREC.UNARY, seq('move', $._unary_expression)),

        copy_expression: $ => prec(PREC.UNARY, seq('copy', $._unary_expression)),

        // ─── Expression terms (postfix and primary) ──────────────────────────────

        _expression_term: $ =>
            choice(
                $.call_expression,
                $.macro_call_expression,
                $.indirect_call_expression,
                $.pack_expression,
                $.name_expression,
                $.generic_name_expression,
                $.dot_expression,
                $.method_call_with_type_args,
                $.index_expression,
                $.cast_expression,
                $.annotate_expression,
                $.is_expression,
                $.block,
                $.vector_expression,
                $._literal_value,
                $.unit_expression,
                $.parenthesized_expression,
                $.expression_list,
                $.break_expression,
                $.continue_expression,
                $.spec_block
            ),

        // Name expression: variable, path, or enum variant
        // Note: type_arguments are NOT on name_expression -- they live on
        // call_expression, pack_expression, dot_expression, vector_expression, etc.
        // This prevents `i < 10` from being mis-parsed as `i<10...>` (name with generic args).
        name_expression: $ => $.name_access_chain,

        // Generic name expression for spec globals: supply<CoinType>
        // Uses prec.dynamic(2) to prefer over comparison interpretation.
        generic_name_expression: $ =>
            prec.dynamic(2, seq($.name_access_chain, field('type_arguments', $.type_arguments))),

        // Function call: foo(), module::foo<T>()
        // The type_arguments variant uses prec.dynamic(2) to win over the comparison
        // interpretation when both parses complete (e.g. exists<T>(addr) vs exists < T).
        // The no-type-args variant uses prec(PREC.CALL) for call vs. expression precedence.
        call_expression: $ =>
            choice(
                prec.dynamic(
                    2,
                    seq(
                        field('function', $.name_access_chain),
                        field('type_arguments', $.type_arguments),
                        field('arguments', $.arg_list)
                    )
                ),
                prec(
                    PREC.CALL,
                    seq(field('function', $.name_access_chain), field('arguments', $.arg_list))
                )
            ),

        // Macro call: assert!(cond, code)
        // The macro name and ! are a single token to avoid ambiguity with NOT operator
        macro_call_expression: $ =>
            prec(
                PREC.CALL,
                seq(
                    field(
                        'macro',
                        alias(token(seq(/[a-zA-Z_][a-zA-Z0-9_]*/, '!')), $.macro_identifier)
                    ),
                    field('arguments', $.arg_list)
                )
            ),

        // Struct/enum pack: Struct { field: value }, Struct<T> { field: value }
        pack_expression: $ =>
            choice(
                // With type arguments -- prec.dynamic(2) to prefer over comparison interpretation
                prec.dynamic(
                    2,
                    seq(
                        field('type', $.name_access_chain),
                        field('type_arguments', $.type_arguments),
                        '{',
                        commaSep($.field_initializer),
                        '}'
                    )
                ),
                // Without type arguments
                seq(field('type', $.name_access_chain), '{', commaSep($.field_initializer), '}')
            ),

        field_initializer: $ =>
            seq(
                field('field', $._field_identifier),
                optional(seq(':', field('value', $._expression)))
            ),

        // Dot expression: obj.field, obj.method(args), obj.0
        dot_expression: $ =>
            prec.left(
                PREC.FIELD,
                seq(
                    field('object', $._expression_term),
                    '.',
                    field('field', choice($.identifier, $.num_literal)),
                    optional(field('arguments', $.arg_list))
                )
            ),

        // Receiver-style method call with type arguments: obj.method<T>(args)
        // Consumes a dot_expression as its object, then extends with type_arguments + arg_list.
        // prec.dynamic(2) makes this win over the binary `<` interpretation when both parses succeed.
        method_call_with_type_args: $ =>
            prec.dynamic(
                2,
                seq(
                    $.dot_expression,
                    field('type_arguments', $.type_arguments),
                    field('arguments', $.arg_list)
                )
            ),

        // Indirect call: (expr)(args), f(a)(b), (self.f)(num)
        indirect_call_expression: $ =>
            prec.left(
                PREC.CALL,
                seq(
                    field(
                        'function',
                        choice(
                            $.parenthesized_expression,
                            $.call_expression,
                            $.indirect_call_expression
                        )
                    ),
                    field('arguments', $.arg_list)
                )
            ),

        // Index expression: vec[i], &T[addr]
        index_expression: $ =>
            prec.left(
                PREC.CALL,
                seq(field('object', $._expression_term), '[', field('index', $._expression), ']')
            ),

        // Type cast: (expr as Type) or expr as Type
        cast_expression: $ =>
            prec.left(PREC.AS, seq(field('expr', $._expression), 'as', field('type', $._type))),

        // Type annotation: (expr : Type)
        annotate_expression: $ =>
            seq('(', field('expr', $._expression), ':', field('type', $._type), ')'),

        // Enum variant test: expr is Variant1 | Variant2, expr is Type<T>
        is_expression: $ =>
            prec.left(
                PREC.EQ,
                seq(field('expr', $._expression), 'is', field('variants', $.is_variant_list))
            ),

        // Variant list for 'is' expression. Each variant may have type arguments.
        is_variant_list: $ => sepBy1('|', $._is_variant),

        _is_variant: $ =>
            choice(
                prec.dynamic(2, seq($.name_access_chain, $.type_arguments)),
                $.name_access_chain
            ),

        // vector literal: vector[1, 2, 3], vector<u8>[]
        // Note: 'vector' is not a keyword -- it's also a valid module/function name
        // We use a prec to prefer this interpretation when followed by [ or <..>[
        vector_expression: $ =>
            prec(
                PREC.CALL + 1,
                seq(
                    alias('vector', $.identifier),
                    optional($.type_arguments),
                    '[',
                    commaSep($._expression),
                    ']'
                )
            ),

        // Parenthesized expression: (expr)
        parenthesized_expression: $ => seq('(', $._expression, ')'),

        // Tuple / grouped expression: (expr, expr)
        expression_list: $ => seq('(', $._expression, ',', commaSep($._expression), ')'),

        // Unit: ()
        unit_expression: _ => seq('(', ')'),

        // Argument list for calls
        arg_list: $ => seq('(', commaSep($._expression), ')'),

        // ─── Block (sequence of statements) ───────────────────────────────────────

        block: $ =>
            seq(
                '{',
                repeat($.use_declaration),
                repeat($._sequence_item),
                optional($._expression),
                '}'
            ),

        _sequence_item: $ => seq(choice($._expression, $.let_expression), ';'),

        // ─── Let expressions ──────────────────────────────────────────────────────

        let_expression: $ =>
            seq(
                'let',
                field('binds', $._bind_list),
                optional(seq(':', field('type', $._type))),
                optional(seq('=', field('value', $._expression)))
            ),

        // ─── Bindings / Patterns ──────────────────────────────────────────────────

        _bind_list: $ => choice($._bind, seq('(', commaSep($._bind), ')')),

        _bind: $ => choice($.bind_var, $.bind_unpack, $.bind_positional_unpack),

        bind_var: $ =>
            choice(
                $.identifier,
                // Contextual keywords that are valid as variable names in let bindings
                alias('exists', $.identifier),
                alias('forall', $.identifier),
                alias('choose', $.identifier),
                alias('package', $.identifier),
                alias('for', $.identifier),
                alias('match', $.identifier),
                // Primitive type keywords used as variable names (e.g. let u8 = 255u8;)
                alias('u8', $.identifier),
                alias('u16', $.identifier),
                alias('u32', $.identifier),
                alias('u64', $.identifier),
                alias('u128', $.identifier),
                alias('u256', $.identifier),
                alias('bool', $.identifier),
                alias('address', $.identifier),
                alias('signer', $.identifier),
                alias('i8', $.identifier),
                alias('i16', $.identifier),
                alias('i32', $.identifier),
                alias('i64', $.identifier),
                alias('i128', $.identifier),
                alias('i256', $.identifier)
            ),

        bind_unpack: $ =>
            seq(
                $.name_access_chain,
                optional($.type_arguments),
                '{',
                commaSep(choice($.bind_field, '..')),
                '}'
            ),

        bind_field: $ =>
            seq(field('field', $.identifier), optional(seq(':', field('bind', $._bind)))),

        bind_positional_unpack: $ =>
            seq(
                $.name_access_chain,
                optional($.type_arguments),
                '(',
                commaSep(choice($._bind, '..')),
                ')'
            ),

        // ─── Quantifier expressions (spec language) ──────────────────────────────

        quantifier_expression: $ =>
            prec.right(
                choice(
                    seq(
                        choice('forall', 'exists'),
                        $.quantifier_bindings,
                        optional(seq('{', commaSep1($._expression), '}')), // trigger/filter annotation
                        optional(seq('where', $._expression)),
                        ':',
                        $._expression
                    ),
                    seq('choose', optional('min'), $.quantifier_binding, 'where', $._expression)
                )
            ),

        quantifier_bindings: $ => commaSep1($.quantifier_binding),

        quantifier_binding: $ =>
            choice(seq($.identifier, ':', $._type), seq($.identifier, 'in', $._expression)),

        // ═══════════════════════════════════════════════════════════════════════════
        // Spec blocks
        // ═══════════════════════════════════════════════════════════════════════════

        spec_block: $ =>
            seq(
                optional($.attributes),
                'spec',
                choice(
                    $._spec_function,
                    seq(optional(field('target', $._spec_block_target)), field('body', $.spec_body))
                )
            ),

        _spec_block_target: $ =>
            choice(
                seq(
                    'fun',
                    $.identifier,
                    optional(field('type_parameters', $.type_parameters)),
                    optional(field('parameters', $.function_parameters)),
                    optional(seq(':', field('return_type', $._type)))
                ),
                seq('struct', $.identifier),
                'module',
                seq('schema', $.identifier, optional($.type_parameters)),
                // bare identifier or module path with optional type params, param bindings, and return type:
                //   spec add { ... }
                //   spec 0x1::coin { ... }
                //   spec initialize(aptos_framework: &signer) { ... }
                //   spec contains(self: &ACL, addr: address): bool { ... }
                //   spec swap<T>(a: &mut T, b: &mut T) { ... }
                seq(
                    $.name_access_chain,
                    optional(field('type_parameters', $.type_parameters)),
                    optional(field('parameters', $.function_parameters)),
                    optional(seq(':', field('return_type', $._type)))
                )
            ),

        spec_body: $ => seq('{', repeat(choice($.use_declaration, $._spec_block_member)), '}'),

        _spec_block_member: $ =>
            choice(
                $.spec_invariant,
                $.spec_condition,
                $.spec_emits,
                $.spec_include,
                $.spec_apply,
                $.spec_pragma,
                $.spec_variable,
                $.spec_let,
                $.spec_update,
                $.spec_axiom,
                $._spec_function,
                $.spec_block // nested spec blocks: spec fun_name(params) { ... }
            ),

        // Spec update: update supply<CoinType> = expr;
        spec_update: $ =>
            seq(
                'update',
                field('name', $.name_access_chain),
                optional(field('type_arguments', $.type_arguments)),
                '=',
                field('value', $._expression),
                ';'
            ),

        // Spec axiom: axiom<T> forall ...: expr;
        spec_axiom: $ =>
            seq(
                'axiom',
                optional(field('type_parameters', $.type_parameters)),
                optional($.condition_properties),
                $._expression,
                ';'
            ),

        spec_invariant: $ =>
            seq(
                'invariant',
                optional(field('type_parameters', $.type_parameters)),
                optional(choice('update', 'pack', 'unpack', 'module')),
                optional($.condition_properties),
                $._expression,
                ';'
            ),

        // Spec emits: emits expr to handle; / emits expr to handle if cond;
        spec_emits: $ =>
            seq(
                'emits',
                $._expression,
                'to',
                $._expression,
                optional(seq('if', $._expression)),
                ';'
            ),

        spec_condition: $ =>
            seq(
                choice(
                    'assert',
                    'assume',
                    'decreases',
                    'ensures',
                    'succeeds_if',
                    seq('requires', optional('module')),
                    'aborts_if',
                    'aborts_with',
                    'modifies'
                ),
                optional($.condition_properties),
                commaSep1($._expression),
                optional(seq('with', $._expression)),
                ';'
            ),

        condition_properties: $ => seq('[', commaSep($.spec_property), ']'),

        spec_property: $ =>
            seq($.identifier, optional(seq('=', choice($._literal_value, $.identifier)))),

        spec_include: $ => seq('include', $._expression, ';'),

        spec_apply: $ =>
            seq(
                'apply',
                $._expression,
                'to',
                commaSep1($.spec_apply_pattern),
                optional(seq('except', commaSep1($.spec_apply_pattern))),
                ';'
            ),

        spec_apply_pattern: $ =>
            seq(
                optional(choice('public', 'internal')),
                field('name_pattern', /[0-9a-zA-Z_*]+/),
                optional($.type_parameters)
            ),

        spec_pragma: $ => seq('pragma', commaSep($.spec_property), ';'),

        spec_variable: $ =>
            seq(
                optional(choice('global', 'local')),
                field('name', $.identifier),
                optional(field('type_parameters', $.type_parameters)),
                ':',
                field('type', $._type),
                optional(seq('=', $._expression)),
                ';'
            ),

        spec_let: $ =>
            seq(
                'let',
                optional('post'),
                field('name', $.identifier),
                '=',
                field('value', $._expression),
                ';'
            ),

        _spec_function: $ =>
            choice(
                $.native_spec_function,
                $.usual_spec_function,
                $.uninterpreted_spec_function,
                // Shorthand: spec name(params): type { body } -- no fun/define keyword
                $.spec_function_shorthand
            ),

        spec_function_shorthand: $ => seq($._spec_function_signature, field('body', $.block)),

        native_spec_function: $ =>
            seq('native', choice('fun', 'define'), $._spec_function_signature, ';'),

        usual_spec_function: $ =>
            seq(choice('fun', 'define'), $._spec_function_signature, field('body', $.block)),

        uninterpreted_spec_function: $ =>
            seq(choice('fun', 'define'), $._spec_function_signature, ';'),

        _spec_function_signature: $ =>
            seq(
                field('name', $.identifier),
                optional(field('type_parameters', $.type_parameters)),
                field('parameters', $.function_parameters),
                ':',
                field('return_type', $._type)
            ),

        // ═══════════════════════════════════════════════════════════════════════════
        // Comments
        // ═══════════════════════════════════════════════════════════════════════════

        line_comment: _ => token(seq('//', /.*/)),

        // Block comments (/* */)
        block_comment: $ =>
            seq(
                '/*',
                optional(
                    choice(
                        // Doc block comments: /** ... */
                        seq($._block_doc_comment_marker, optional($._block_comment_content)),
                        // Regular block comments
                        $._block_comment_content
                    )
                ),
                '*/'
            ),

        // ═══════════════════════════════════════════════════════════════════════════
        // Identifier
        // ═══════════════════════════════════════════════════════════════════════════

        identifier: _ => /[a-zA-Z_][a-zA-Z0-9_]*/,
    },
});
