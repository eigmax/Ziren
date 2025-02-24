# Offline memory checking in zkVM

Offline memory checking is a method that enables a prover to demonstrate to a verifier that a read/write memory was used correctly. In such a memory system, values \\(v\\) can be written to an addresses \\(a\\) and subsequently retrieved. This technique allows the verifier to efficiently confirm that the prover adhered to the memory's rules (i.e., that the value returned by any read operation is indeed the most recent value that was written to that memory adderss).

The term "offline memory checking" refers to techniques that check the correctness of all read operations "all at once", after the reads have all occurred--or in zkVM settings, after the purported values returned by the reads have been committed. Off-line checking techniques do not determine as a read happens whether or not it was correct. They only ascertain, when all the reads are checked at once, whether or not all of the reads were correct.

This is in contrast to "online memory checking" techniques like Merkle hashing that immediately confirm that a memory read was done correctly by insisting that each read includes an authentication path. Merkle hashing is much more expensive on a per-read basis for ZK provers, and offline memory checking suffices for zkVM design.

## Construction of read set $RS$ and write set \\(WS\\) 

Here we define two sets read set \\(RS\\) and write set \\(WS\\), which are sets of tuples (address \\(a\\), value \\(v\\), op count \\(c\\)). We construct them according to the following steps.

Initialization:

- \\(RS = WS = \emptyset\\);
- All memory cells \\(a_i\\) are initialized with some value \\(v_i\\) at op count \\(c=0\\). Add the initial tuples to the write set \\(WS = WS \bigcup \\{(a_i, v_i, 0)\\}\\) for all \\(i\\).

Read and write operations:

- For reading a value from address \\(a\\), find the last tuple \\((a, v, c)\\) added to write set \\(WS\\) with the address \\(a\\). Set \\(RS = RS \bigcup \\{(a, v, c)\\}\\) and \\(WS = WS \bigcup \\{(a, v, c_{now})\\}\\), with \\(c_{now}\\) the current op count.
- For writing a value \\(v'\\) to address \\(a\\), find the last tuple \\((a, v, c)\\) added to write set \\(WR\\) with the address \\(a\\). Set \\(RS = RS \bigcup \\{(a, v, c)\\}\\) and \\(WS = WS \bigcup \\{(a, v', c_{now})\\}\\).

Post-processing：

- For all memory cells \\(a_i\\), add the last tuple \\((a_i, v_i, c_i)\\) in write set \\(WS\\) to \\(RS\\), \\(RS = RS \bigcup \\{(a_i, v_i, c_i)\\}\\).

## Core observation

If the following restrictions are applied, the prover adheres to the memory rules, ensuring that the correct values are read from memory.

1) The read and write sets are correctly initialized; 
2) For each address \\(a_i\\), the instruction count added to \\(WS\\) strictly increases over time;
3) Tuples added to the read set and write set for read operations must have the same value; tuples added to the read set and write set for read-write operations must satisfy that the op count of the read set is less than that of the write set;
4) After post-processing, \\(RS = WS\\).

Brief Proof: Consider the first erroneous read memory operation. Assume that a read operation was expected to return the tuple \\((a,v,c)\\), but it actually returned an incorrect tuple \\((a, v' \neq v, c')\\) and added it to read set \\(RS\\). Note that all tuples in \\(WS\\) are distinct. After adding \\((a,v',c_{now})\\) to \\(WS\\), the tuples \\((a,v,c)\\) and \\((a,v',c_{now})\\) are not in the read set \\(RS\\). According to restriction 3, after each read-write operation, there are always at least two tuples in \\(WS\\) that are not in \\(RS\\), making it impossible to adjust to \\(RS = WS\\) through post-processing.

There are two different ways to prove \\(RS = WS\\), multiset hashing method and lookup table mehtod (e.g., LogUp) respectively.

## Multiset hashing method

Multiset hashing allows a (multi-)set to be hashed into a short string, making it computationally difficult to find two different sets that hash to the same value. The hashing is performed incrementally — one element at a time. A key property is that the hash value is independent of the order in which elements are hashed. Therefore, we can compare sets generated in different orders.

Consider the group \\(G\\) as the set of points \\((x,y)\\) on the elliptic curve \\(y^2 = x^3 +Ax+B\\) (including the point at infinity). We can implement a hash-to-group approach. To hash a set element into a point on the elliptic curve, we first map the set element to the \\(x\\)-coordinate of the point. Since this may not be a valid \\(x\\)-coordinate on the elliptic curve, we add an 8-bit tweak \\(t\\). Additionally, we constrain the sign of the \\(y\\)-coordinate to prevent flipping, either by ensuring \\(y\\) is a quadratic residue or by adding range checks.

For example, in a zkVM execution, we can construct the following columns and corresponding constraints.

Columns:

- \\(a\\): address of the operation;
- \\(v_r, v_w\\): values added to the read/write sets;
- \\(c_r, c_w\\): op counters added to the read/write sets;
- \\(f_r, f_w\\): mapping of a set element to a single value, ensuring the mapping is injective (e.g., if \\(a, v, c\\) are all within 32 bits, we can use \\(f = a << 72 + v << 40 + c << 8\\));
- \\(t_r, t_w\\):  8-bit tweak;
- \\(x_r, x_w\\): \\(x\\)-coordinate, satisfying \\(x = f + t\\);
- \\(y_r, y_w\\): \\(y\\)-coordinate;
- \\(z_r, z_w\\): satisfying \\(y = z^2\\), ensuring \\(y\\) is a quadratic residue;
- \\(h_r, h_w\\): current hash value of the read/write sets.

 Constraints:

1) Range check for  \\(a, v, c, t\\);
2) \\(c_{now} = c_w > c_r\\);
3) \\(x= a << 72 + v << 40 + c << 8 + t, y^2 = x^3 + A x + B, y = z^2\\);
4) \\(h = h_{old} + (x,y)\\);
5) Finally \\(h_r = h_w\\).

## Logup method

To prove two multisets \\(A = \\{a_i\\}\\) and \\(B = \{b_i\}\\), it suffices to prove that for a random selected \\(\gamma\\), the following holds:
\\[\sum_i \frac{1}{a_i+\gamma} = \sum_i \frac{1}{b_i+\gamma}\\]

We can construct the following columns and constraints.

Columns:

- \\(a\\);
- \\(v_r, v_w\\);
- \\(c_r, c_w\\);
- \\(f_r, f_w\\);
- \\(aux_r, aux_w\\): \\(aux = \frac{1}{f + \gamma}\\); 
- \\(s_r, s_w\\): partial sums \\(s = \sum aux\\).

Constraints:
1) Range checks for \\(a, v, c\\);
2) \\(c_{now} = c_w > r_w\\);
3) \\(f = f(a,v,c)\\) is an injective mapping;
4) \\(aux \cdot (f + \gamma) = 1\\);
5) \\(s = s_{old} + aux\\);
6) Finally, \\(s_r = s_w\\).

Note: Before committing to \\(a, v, c\\)(or their combination \\(f\\)), we don't have the challenge \\(\gamma\\).  Therefore, we need a two-pass method: first run the program, commit to \\(f\\), and derive \\(\gamma\\); then run the program again to compute \\(aux\\) and \\(s\\). 

## Comparison of the Two Methods

Multiset Hashing:

- Requires only one execution of the program to commit to all columns.

- Disadvantage: Involves more columns and more complex constraints.

LogUp Method:

- Requires two passes to commit to different columns.

- Advantage: Involves fewer columns and simpler constraints.