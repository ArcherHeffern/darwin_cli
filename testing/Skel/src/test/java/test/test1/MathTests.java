package test.test1;

import static org.junit.Assert.assertEquals;
import org.junit.Test;


public class MathTests {

	private static final double DELTA = 0.01;

	@Test
	public void testAdd(){
		assertEquals(5, Math.add(2, 3), DELTA);
		assertEquals(-5, Math.add(7, -12), DELTA);
	}

	@Test
	public void testSub(){
		assertEquals(-1, Math.sub(2, 3), DELTA);
		assertEquals(19, Math.sub(7, -12), DELTA);
	}
	
	@Test
	public void testMul(){
		assertEquals(6, Math.mul(2, 3), DELTA);
		assertEquals(-84, Math.mul(7, -12), DELTA);
	}

	@Test
	public void testDiv(){
		assertEquals(2.0/3.0, Math.div(2, 3), DELTA);
		assertEquals(7.0/-12.0, Math.div(7, -12), DELTA);
	}
}
