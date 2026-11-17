import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import NewBattle from './NewBattle';

const sizes: Record<string, [number, number]> = { '4:3': [800, 600], '2:1': [1200, 600], '16:9': [1067, 600], '1:1': [600, 600] };
const sizeOf = (r: string) => sizes[r];

describe('NewBattle ratio picker', () => {
  it('lists the four ratios with their sizes, 4:3 first', () => {
    render(<NewBattle names={['A', 'B']} onRun={() => {}} sizeOf={sizeOf} ratios={Object.keys(sizes)} />);
    const labels = screen.getAllByRole('option').map((o) => o.textContent).filter((t) => t?.includes('×'));
    expect(labels).toEqual(['4:3 · 800 × 600', '2:1 · 1200 × 600', '16:9 · 1067 × 600', '1:1 · 600 × 600']);
  });

  it('runs with the size of the picked ratio', () => {
    const onRun = vi.fn();
    render(<NewBattle names={['A', 'B']} onRun={onRun} sizeOf={sizeOf} ratios={Object.keys(sizes)} />);
    fireEvent.change(screen.getByLabelText('Ratio'), { target: { value: '16:9' } });
    fireEvent.click(screen.getByText('Run battle'));
    expect(onRun).toHaveBeenCalledWith(expect.objectContaining({ width: 1067, height: 600 }));
  });
});
