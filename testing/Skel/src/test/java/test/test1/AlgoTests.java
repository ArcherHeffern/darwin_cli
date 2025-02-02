package test.test1;

import static org.junit.Assert.assertEquals;
import org.junit.Test;


public class AlgoTests {

	@Test
	public void testFibIter() {
		assertEquals(0, Algo.fibIter(1));
		assertEquals(1, Algo.fibIter(2));
		assertEquals(1, Algo.fibIter(3));
		assertEquals(2, Algo.fibIter(4));
		assertEquals(3, Algo.fibIter(5));
		assertEquals(5, Algo.fibIter(6));
	}
}
