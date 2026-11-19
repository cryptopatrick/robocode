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

  it('defaults to Tactician against Circler', () => {
    const onRun = vi.fn();
    render(<NewBattle names={['Tactician', 'Circler', 'Spinner']} onRun={onRun} sizeOf={sizeOf} ratios={Object.keys(sizes)} />);
    fireEvent.click(screen.getByText('Run battle'));
    expect(onRun).toHaveBeenCalledWith(expect.objectContaining({ robots: ['Tactician', 'Circler'] }));
  });
});

describe('NewBattle tank picker', () => {
  const setup = (onRun = vi.fn()) => {
    render(<NewBattle names={['A', 'B', 'C']} onRun={onRun} sizeOf={sizeOf} ratios={Object.keys(sizes)} />);
    return onRun;
  };

  it('each robot row has a tank picker with slot defaults', () => {
    setup();
    const shown = (l: string) => (screen.getByLabelText(l) as HTMLSelectElement).selectedOptions[0].textContent;
    expect(shown('Tank for robot 1')).toBe('Leopard 2A8');
    expect(shown('Tank for robot 2')).toBe('M1A2 Abrams SEPv3');
  });

  it('picked tanks reach onRun', () => {
    const onRun = setup();
    fireEvent.change(screen.getByLabelText('Tank for robot 2'), { target: { value: 'k2' } });
    fireEvent.click(screen.getByText('Run battle'));
    expect(onRun).toHaveBeenCalledWith(expect.objectContaining({ tanks: ['leopard2a8', 'k2'] }));
  });

  it('removing_a_robot_keeps_other_tanks', () => {
    const onRun = setup();
    fireEvent.click(screen.getByText('Add robot'));
    fireEvent.change(screen.getByLabelText('Tank for robot 2'), { target: { value: 'k2' } });
    fireEvent.change(screen.getByLabelText('Tank for robot 3'), { target: { value: 'merkava' } });
    fireEvent.click(screen.getByLabelText('Remove robot 2'));
    fireEvent.click(screen.getByText('Run battle'));
    expect(onRun).toHaveBeenCalledWith(expect.objectContaining({ tanks: ['leopard2a8', 'merkava'] }));
  });

  it('adding a robot gets the next slot default', () => {
    const onRun = setup();
    fireEvent.click(screen.getByText('Add robot'));
    fireEvent.click(screen.getByText('Run battle'));
    expect(onRun).toHaveBeenCalledWith(expect.objectContaining({ tanks: ['leopard2a8', 'abrams', 'k2'] }));
  });
});

describe('NewBattle bullet picker', () => {
  const setup = (onRun = vi.fn()) => {
    render(<NewBattle names={['A', 'B']} onRun={onRun} sizeOf={sizeOf} ratios={Object.keys(sizes)} />);
    return onRun;
  };

  it('defaults to Realistic', () => {
    const onRun = setup();
    expect((screen.getByLabelText('Bullets') as HTMLSelectElement).selectedOptions[0].textContent).toBe('Realistic');
    fireEvent.click(screen.getByText('Run battle'));
    expect(onRun).toHaveBeenCalledWith(expect.objectContaining({ bullets: 'realistic' }));
  });

  it('the picked mode reaches onRun', () => {
    const onRun = setup();
    fireEvent.change(screen.getByLabelText('Bullets'), { target: { value: 'arcade' } });
    fireEvent.click(screen.getByText('Run battle'));
    expect(onRun).toHaveBeenCalledWith(expect.objectContaining({ bullets: 'arcade' }));
  });
});
