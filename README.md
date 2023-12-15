Processing a stream of hashes

1. Bloom Filter Pre-Filtering
    - only add the element to the queue if it might be in the dataset
2. Sort Queue
    - after a certain threshold is reached sort the queue for the next step
3. Search the elements of the queue
    - use adaptive interpolation and binary search to determine if/where an element is located
    - retain the bounds of adaptive interpolation and adjust it as elements of the queue are processed
4. Repeat until end of Stream
    - go back to step 1 until all elements have been processed

The explicit switching between filtering which elements are even considered and fully determining if/where the element is in the dataset is done to optimize cache efficiency. During the filtering step, memory for that step will reside in the caches and will not be evicted by the explicit searches. Once the searching starts the memory involved in finding the elements will primarily be what's in the cache. I believe that this process could potentially be run in parallel for even better performance, or maybe suffer from resource conflict because this problem is mostly memory bound.

Initalize bloom filter upon use rather than pre-compute. A bloom filter may not be worth while at all if it's often saturated or the bit array is so large that cacheing is ineffective.


