import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import LiveControls, { type LiveUi } from './LiveControls';

const ui = (over: Partial<LiveUi> = {}): LiveUi => ({
  paused: false, tps: 30, measuredTps: 29.6, finished: false, following: true, ...over,
});

const handlers = () => ({
  onPause: vi.fn(), onResume: vi.fn(), onStep: vi.fn(), onTps: vi.fn(), onSeek: vi.fn(), onBackToLive: vi.fn(),
});

describe('LiveControls', () => {
  it('shows the live badge and measured vs target speed', () => {
    render(<LiveControls live={ui()} turn={40} last={40} {...handlers()} />);
    expect(screen.getByText('LIVE')).toBeTruthy();
    expect(screen.getByText('30 / 30 turns/s')).toBeTruthy();
    expect(screen.getByText('Turn 40 / 40')).toBeTruthy();
  });

  it('pauses, then resumes and steps while paused', () => {
    const h = handlers();
    const { rerender } = render(<LiveControls live={ui()} turn={5} last={5} {...h} />);
    expect((screen.getByRole('button', { name: 'Step forward' }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
    expect(h.onPause).toHaveBeenCalled();
    rerender(<LiveControls live={ui({ paused: true })} turn={5} last={5} {...h} />);
    fireEvent.click(screen.getByRole('button', { name: 'Step forward' }));
    expect(h.onStep).toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Resume' }));
    expect(h.onResume).toHaveBeenCalled();
  });

  it('offers max speed', () => {
    const h = handlers();
    render(<LiveControls live={ui()} turn={5} last={5} {...h} />);
    fireEvent.change(screen.getByLabelText('Speed'), { target: { value: 'max' } });
    expect(h.onTps).toHaveBeenCalledWith(null);
  });

  it('offers a way back to live after scrubbing', () => {
    const h = handlers();
    render(<LiveControls live={ui({ following: false })} turn={12} last={40} {...h} />);
    fireEvent.click(screen.getByRole('button', { name: 'Back to live' }));
    expect(h.onBackToLive).toHaveBeenCalled();
  });

  it('says when the battle has finished', () => {
    render(<LiveControls live={ui({ finished: true })} turn={90} last={90} {...handlers()} />);
    expect(screen.getByText('Finished')).toBeTruthy();
  });
});
